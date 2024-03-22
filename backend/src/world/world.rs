use std::{collections::HashMap, mem::size_of, rc::Rc};

use glam::{ivec3, uvec3, IVec3, UVec3, Vec3};
use glow::{HasContext, WebVertexArrayKey};
use log::info;

const MAX_LOAD_CHUNK_DISTANCE: i32 = 5;
const MAX_LOAD_CHUNK_DISTANCE_SQUARED: i32 = MAX_LOAD_CHUNK_DISTANCE * MAX_LOAD_CHUNK_DISTANCE;
const MAX_MESH_TO_KEEP: usize = 1000;

// const CHUNK_LOADING_WEIGHT: f32 = 1.0;
// const CHUNK_GENERATION_WEIGHT: f32 = 0.5;
const MAX_LOADS_PER_FRAME: usize = 16;

use crate::{
    core::Time,
    graphics::Camera,
    graphics::TextureType,
    graphics::{ShaderDef, ShaderProgram, UniformTypes},
    shader_def,
    world::{Chunk, ChunkGenerator, CHUNK_SIZE},
};

use super::chunk;

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

type OffsetPriority = Vec<IVec3>;

fn make_offset_priority(size: i32) -> OffsetPriority {
    let total_offsets = size * size * size;
    let mut offsets = Vec::with_capacity(total_offsets as _);
    for z in -size + 1..size {
        for y in -size + 1..size {
            for x in -size + 1..size {
                offsets.push(ivec3(x, y, z));
            }
        }
    }
    offsets.sort_by(|p, q| p.length_squared().cmp(&q.length_squared()));
    info!(
        "offset: {}, {}, ..., {}, {}",
        offsets[0],
        offsets[1],
        offsets[total_offsets as usize - 2],
        offsets[total_offsets as usize - 1]
    );
    offsets
}

#[derive(Debug)]
pub struct World<G>
where
    G: ChunkGenerator,
{
    chunks: HashMap<IVec3, CurrentChunk>,
    loaded_vertices: usize,
    // loaded_meshes: Vec<LoadedChunkMesh>,
    graphics: Option<GraphicContext>,
    generator: G,
    last_computed_chunk_pos: Option<IVec3>,
    offset_priority: OffsetPriority,
    chunks_to_load: Vec<IVec3>,
}

impl<G> World<G>
where
    G: ChunkGenerator,
{
    pub fn new(generator: G) -> Self {
        info!("Creating world");
        let offset_priority = make_offset_priority(MAX_LOAD_CHUNK_DISTANCE);

        Self {
            chunks: HashMap::new(),
            graphics: None,
            loaded_vertices: 0,
            generator,
            offset_priority,
            last_computed_chunk_pos: None,
            chunks_to_load: Vec::with_capacity(1000),
        }
    }

    pub fn get_info(&self) -> String {
        let vertex_count = self.loaded_vertices;
        let memory_used = (vertex_count * size_of::<f32>()) as f32 / 1000000.0;
        let loaded_meshes = self.chunks.values().filter(|c| c.mesh.is_some()).count();
        format!(
            "{}/{} chunks\n{vertex_count} vertices - {memory_used:.3} MB",
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
        let player_block_pos = player_pos.as_ivec3();
        let player_chunk_pos = player_block_pos / CHUNK_SIZE as i32;

        if self.last_computed_chunk_pos != Some(player_chunk_pos) {
            info!("Recompute for chunk pos: {player_chunk_pos:?}");

            let mut chunks_we_can_unload = Vec::with_capacity(1000);
            for c in self.chunks.values() {
                if let Some(_) = &c.mesh {
                    let chunk_pos = c.chunk.chunk_position;
                    if player_chunk_pos.distance_squared(chunk_pos)
                        > MAX_LOAD_CHUNK_DISTANCE_SQUARED
                    {
                        chunks_we_can_unload.push(chunk_pos);
                    }
                }
            }
            chunks_we_can_unload.sort_by(|a, b| {
                a.distance_squared(player_chunk_pos)
                    .cmp(&b.distance_squared(player_chunk_pos))
            });
            if chunks_we_can_unload.len() > MAX_MESH_TO_KEEP {
                info!(
                    "Unloading {} chunks",
                    chunks_we_can_unload.len() - MAX_MESH_TO_KEEP
                );
                for chunk_pos in chunks_we_can_unload.drain(MAX_MESH_TO_KEEP..) {
                    self.chunks.get_mut(&chunk_pos).unwrap().mesh = None;
                }
            }

            let player_chunk_pos = player_block_pos / CHUNK_SIZE as i32;
            for offset in &self.offset_priority {
                let chunk_pos = player_chunk_pos + *offset;
                let chunk = self.chunks.get(&chunk_pos);
                if chunk.is_none() || chunk.unwrap().mesh.is_none() {
                    self.chunks_to_load.push(chunk_pos);
                }
            }
            self.last_computed_chunk_pos = Some(player_chunk_pos);
        }
        let chunks_to_load = self.chunks_to_load.len();
        if chunks_to_load > 0 {
            let chunks_to_load = std::cmp::min(chunks_to_load, MAX_LOADS_PER_FRAME);
            info!("Loading {chunks_to_load} chunks this frame");
            for chunk_pos in self.chunks_to_load.drain(0..chunks_to_load) {
                let chunk = self.chunks.get(&chunk_pos);
                if !chunk.is_some() {
                    self.chunks.insert(
                        chunk_pos,
                        CurrentChunk {
                            chunk: self.generator.generate(&chunk_pos),
                            mesh: None,
                        },
                    );
                }
                self.chunks.entry(chunk_pos).and_modify(|c| {
                    c.mesh = Some(LoadedChunkMesh::load(gl, &c.chunk).expect("can't load mesh"));
                });
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
                gl.enable(glow::CULL_FACE);
                gl.enable(glow::DEPTH_TEST);
                gl.disable(glow::BLEND);
                for c in self.chunks.values() {
                    let chunk = &c.chunk;
                    if let Some(mesh) = &c.mesh {
                        let world_pos = chunk.chunk_position.as_vec3() * CHUNK_SIZE as f32;
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
