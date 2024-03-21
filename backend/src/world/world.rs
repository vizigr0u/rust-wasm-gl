use std::{collections::HashMap, mem::size_of, rc::Rc};

use glam::{ivec3, uvec3, IVec3, UVec3, Vec3};
use glow::{HasContext, WebVertexArrayKey};
use log::info;

const MAX_CHUNKS_GENERATED_PER_FRAME: usize = 32;
const MAX_CHUNKS_LOADED_PER_FRAME: usize = 16;

const CHUNK_LOADING_WEIGHT: f32 = 1.0;
const CHUNK_GENERATION_WEIGHT: f32 = 0.5;
const MAX_LOAD_PER_FRAME: f32 = 16.0;

use crate::{
    core::Time,
    graphics::Camera,
    graphics::TextureType,
    graphics::{ShaderDef, ShaderProgram, UniformTypes},
    shader_def,
    world::{Chunk, ChunkGenerator, CHUNK_SIZE},
};

#[derive(Debug)]
struct GraphicContext {
    program: Rc<ShaderProgram>,
    texture: Rc<(TextureType, glow::WebTextureKey)>,
}

#[derive(Debug)]
struct LoadedChunkMesh {
    pub vertex_array: WebVertexArrayKey,
    pub vertex_count: usize,
}

#[derive(Debug)]
struct CurrentChunk {
    chunk: Chunk,
    mesh: Option<LoadedChunkMesh>,
}

#[derive(Debug)]
pub struct World<G>
where
    G: ChunkGenerator,
{
    chunks: HashMap<IVec3, CurrentChunk>,
    size: UVec3,
    loaded_vertices: usize,
    // loaded_meshes: Vec<LoadedChunkMesh>,
    graphics: Option<GraphicContext>,
    generator: G,
}

impl<G> World<G>
where
    G: ChunkGenerator,
{
    pub fn random(size: UVec3, generator: G) -> Self {
        let num_chunks = (size.x * size.y * size.z) as usize;
        info!("Creating world with {} chunks", num_chunks);

        Self {
            chunks: HashMap::new(),
            size,
            graphics: None,
            loaded_vertices: 0,
            generator,
        }
    }

    pub fn get_info(&self) -> String {
        let max_chunks = (self.size.x * self.size.y * self.size.z) as usize;
        if self.chunks.len() < max_chunks {
            return format!("Loading...{}/{}", self.chunks.len(), max_chunks);
        }
        let vertex_count = self.loaded_vertices;
        let memory_used = (vertex_count * size_of::<f32>()) as f32 / 1000000.0;
        let loaded_meshes = self.chunks.values().filter(|c| c.mesh.is_some()).count();
        format!(
            "size: {}x{}x{} - {}/{} chunks\n{vertex_count} vertices - {memory_used:.3} MB",
            self.size.x,
            self.size.y,
            self.size.z,
            loaded_meshes,
            self.chunks.len(),
        )
    }

    pub fn setup_graphics(
        &mut self,
        gl: &glow::Context,
        texture: Rc<(TextureType, glow::WebTextureKey)>,
    ) -> Result<(), String> {
        let program = compile_shader(gl)?;
        self.graphics = Some(GraphicContext { program, texture });
        Ok(())
    }

    // pub fn load_all(&mut self, gl: &glow::Context) -> Result<(), String> {
    //     self.bake_chunks(gl, self.chunks.len())
    // }

    // fn bake_chunks(&mut self, gl: &glow::Context, max_count: usize) -> Result<(), String> {
    //     let start_index = self.loaded_meshes.len();
    //     let mut max_index = start_index + max_count;
    //     if max_index > self.chunks.len() {
    //         max_index = self.chunks.len();
    //     }
    //     for i in start_index..max_index {
    //         let chunk = &self.chunks[i];
    //         let mesh = LoadedChunkMesh::load(gl, chunk)?;
    //         self.loaded_meshes.push(mesh)
    //     }
    //     self.loaded_vertices = self.loaded_meshes.iter().map(|m| m.vertex_count).sum();
    //     Ok(())
    // }

    // fn generate_chunks(&mut self, max_count: usize) {
    //     let start_index = self.chunks.len();
    //     let mut max_index = start_index + max_count;
    //     let max_chunk_count = (self.size.x * self.size.y * self.size.z) as _;
    //     if max_index > max_chunk_count {
    //         max_index = max_chunk_count;
    //     }
    //     for i in start_index..max_index {
    //         let i = i as u32;
    //         let x = i % self.size.x;
    //         let y = (i / self.size.x) % self.size.y;
    //         let z = (i / self.size.x) / self.size.y;
    //         let chunk: Chunk = self.generator.generate(&UVec3::new(x, y, z));
    //         self.chunks.push(chunk);
    //     }
    // }

    pub fn update(
        &mut self,
        gl: &glow::Context,
        _time: &Time,
        player_pos: Vec3,
    ) -> Result<(), String> {
        let player_pos = (player_pos / CHUNK_SIZE as f32).as_ivec3();
        let max_dist = ivec3(15, 2, 15);
        for c in self.chunks.values_mut() {
            if let Some(_) = &c.mesh {
                if (c.chunk.world_position.x - player_pos.x).abs() > max_dist.x
                    || (c.chunk.world_position.y - player_pos.y).abs() > max_dist.y
                    || (c.chunk.world_position.z - player_pos.z).abs() > max_dist.z
                {
                    c.mesh = None;
                }
            }
        }
        let mut loading_budget = MAX_LOAD_PER_FRAME;
        for x in 0..=max_dist.x {
            for y in 0..=max_dist.y {
                for z in 0..=max_dist.z {
                    for p in [player_pos - ivec3(x, y, z), player_pos + ivec3(x, y, z)] {
                        if loading_budget <= 0.0 {
                            info!("budget exhausted");
                            return Ok(());
                        }
                        if self.chunks.get(&p).is_none() {
                            info!("generating chunk {}", p);
                            loading_budget -= CHUNK_GENERATION_WEIGHT;
                            self.chunks.insert(
                                p,
                                CurrentChunk {
                                    chunk: self.generator.generate(&p),
                                    mesh: None,
                                },
                            );
                        }
                        if loading_budget > 0.0
                            && self.chunks.get(&p).is_some_and(|c| c.mesh.is_none())
                        {
                            self.chunks.entry(p).and_modify(|c| {
                                info!("loading chunk {}", p);
                                loading_budget -= CHUNK_LOADING_WEIGHT;
                                c.mesh = Some(
                                    LoadedChunkMesh::load(gl, &c.chunk).expect("can't load mesh"),
                                );
                            });
                        }

                        if z == 0 && y == 0 && x == 0 {
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn render(&self, gl: &glow::Context, camera: &Camera) {
        if let Some(graphics) = &self.graphics {
            unsafe {
                let program = &graphics.program;
                program.gl_use(gl);
                gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(graphics.texture.1));
                program.set_matrix(gl, UniformTypes::ViewMatrix, &camera.look_at);
                program.set_matrix(gl, UniformTypes::ProjMatrix, &camera.projection);
                let world_pos_position = program.get_uniform_location(UniformTypes::WorldPosition);

                for c in self.chunks.values() {
                    let chunk = &c.chunk;
                    if let Some(mesh) = &c.mesh {
                        let world_pos = chunk.world_position.as_vec3() * CHUNK_SIZE as f32;
                        gl.uniform_3_f32(world_pos_position, world_pos.x, world_pos.y, world_pos.z);

                        gl.bind_vertex_array(Some(mesh.vertex_array));
                        gl.draw_arrays(glow::TRIANGLES, 0, mesh.vertex_count as _);
                    }
                }
            }
        }
    }
}

impl LoadedChunkMesh {
    fn load(gl: &glow::Context, chunk: &Chunk) -> Result<Self, String> {
        info!("LoadedChunkMesh loading chunk {}", chunk.world_position);
        let vertex_data = chunk.to_vertex_data();
        unsafe {
            let vao = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vao));
            let vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertex_data.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            let location = 0;
            let size = 1;
            gl.vertex_attrib_pointer_i32(location, size, glow::INT, 0, 0);
            gl.enable_vertex_attrib_array(location);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            Ok(Self {
                vertex_array: vao,
                vertex_count: vertex_data.len(),
            })
        }
    }
}

fn compile_shader(gl: &glow::Context) -> Result<Rc<ShaderProgram>, String> {
    unsafe {
        let program = shader_def!(
            "chunk.vert",
            "chunk.frag",
            vec!(),
            vec!(
                (UniformTypes::WorldPosition, "world_pos"),
                (UniformTypes::ViewMatrix, "view"),
                (UniformTypes::ProjMatrix, "projection"),
            )
        )
        .compile(gl)?;
        Ok(Rc::new(program))
    }
}
