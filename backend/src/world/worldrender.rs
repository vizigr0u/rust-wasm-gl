use std::{cell::Cell, collections::HashMap, rc::Rc};

use glow::{HasContext, WebBufferKey, WebVertexArrayKey};

use crate::{
    graphics::{Camera, ShaderDef, ShaderProgram, TextureType, UniformTypes},
    shader_def,
    world::{Chunk, Neighbours, Side, CHUNK_SIZE},
};

use super::{make_base_quad_data, ChunkPos};

#[derive(Debug)]
struct GraphicContext {
    program: Rc<ShaderProgram>,
    texture: Rc<(TextureType, glow::WebTextureKey)>,
}

#[derive(Debug, Clone)]
pub struct ChunkDrawData {
    pub vertex_data: [Vec<i32>; 6],
}

/// One chunk's quads on the GPU: all 6 sides concatenated in one buffer.
#[derive(Debug)]
struct GpuChunk {
    buffer: WebBufferKey,
    /// (first quad, quad count) per `Side`.
    side_ranges: [(i32, i32); 6],
}

const DATA_ATTR: u32 = 1;
const OFFSET_ATTR: u32 = 2;

const SIDES: [Side; 6] = [
    Side::Top,
    Side::Bottom,
    Side::Front,
    Side::Back,
    Side::Right,
    Side::Left,
];

#[derive(Debug)]
pub struct WorldRenderData {
    graphics: GraphicContext,
    vertex_array: WebVertexArrayKey,
    gpu_chunks: HashMap<ChunkPos, GpuChunk>,
    /// Draw calls issued by the last `render` (kept by both backends).
    draw_calls: Cell<u32>,
}

impl ChunkDrawData {
    pub fn quad_count(&self) -> usize {
        self.vertex_data.iter().map(Vec::len).sum()
    }

    pub fn load(chunk: &Chunk, neighbours: &Neighbours) -> Self {
        Self {
            vertex_data: chunk.to_vertex_data(neighbours),
        }
    }
}

impl WorldRenderData {
    pub fn get_info(&self) -> String {
        format!(
            "GPU: {} chunk buffers, {} draw calls",
            self.gpu_chunks.len(),
            self.draw_calls.get()
        )
    }

    // INTENTIONAL: cull faces relative to the *player* chunk, not the camera.
    // The orbit camera is a debug view: holes seen from far away show what the
    // player's own view culls. Do not "fix" (bugs.md #19) until a player-attached
    // camera exists; then this debug camera can keep showing the culling.
    fn is_visible(player_chunk_pos: ChunkPos, data_pos: ChunkPos, side: Side) -> bool {
        match side {
            Side::Front => player_chunk_pos.as_vec().z >= data_pos.as_vec().z,
            Side::Back => player_chunk_pos.as_vec().z - 1 <= data_pos.as_vec().z,
            Side::Left => player_chunk_pos.as_vec().x - 1 <= data_pos.as_vec().x,
            Side::Right => player_chunk_pos.as_vec().x >= data_pos.as_vec().x,
            Side::Top => player_chunk_pos.as_vec().y >= data_pos.as_vec().y,
            Side::Bottom => player_chunk_pos.as_vec().y - 1 <= data_pos.as_vec().y,
        }
    }

    /// Create or replace the GPU buffer of one chunk.
    pub fn upload_chunk(&mut self, gl: &glow::Context, pos: ChunkPos, data: &ChunkDrawData) {
        self.remove_chunk(gl, pos);
        if data.quad_count() == 0 {
            return;
        }
        let mut all: Vec<i32> = Vec::with_capacity(data.quad_count());
        let mut side_ranges = [(0, 0); 6];
        for side in SIDES {
            let quads = &data.vertex_data[side as usize];
            side_ranges[side as usize] = (all.len() as i32, quads.len() as i32);
            all.extend_from_slice(quads);
        }
        unsafe {
            let Ok(buffer) = gl.create_buffer() else {
                log::error!("World: could not create chunk buffer");
                return;
            };
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                all.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            self.gpu_chunks.insert(
                pos,
                GpuChunk {
                    buffer,
                    side_ranges,
                },
            );
        }
    }

    pub fn remove_chunk(&mut self, gl: &glow::Context, pos: ChunkPos) {
        if let Some(chunk) = self.gpu_chunks.remove(&pos) {
            unsafe { gl.delete_buffer(chunk.buffer) };
        }
    }

    pub fn new(
        gl: &glow::Context,
        texture: Rc<(TextureType, glow::WebTextureKey)>,
    ) -> Result<Self, String> {
        let program = compile_shader(gl)?;

        unsafe {
            program.gl_use(gl);

            let vertex_array = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vertex_array));

            // upload quad data
            let vertices = make_base_quad_data();
            let vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );
            let mesh_pos_location = 0;
            gl.vertex_attrib_pointer_f32(mesh_pos_location, 3, glow::FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(mesh_pos_location);

            // vertex data attributes - 1 per quad
            gl.enable_vertex_attrib_array(DATA_ATTR);
            gl.vertex_attrib_divisor(DATA_ATTR, 1);

            // attribute 2 (chunk offset from player) is a constant set per draw call
            gl.disable_vertex_attrib_array(OFFSET_ATTR);

            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_vertex_array(None);

            Ok(Self {
                graphics: GraphicContext { program, texture },
                vertex_array,
                gpu_chunks: HashMap::new(),
                draw_calls: Cell::new(0),
            })
        }
    }

    pub fn render(&self, gl: &glow::Context, camera: &Camera, player_chunk_pos: ChunkPos) {
        let graphics = &self.graphics;
        unsafe {
            let program = &graphics.program;
            program.gl_use(gl);
            gl.bind_texture(graphics.texture.0.into(), Some(graphics.texture.1));
            program.set_matrix(gl, UniformTypes::ViewMatrix, &camera.look_at);
            program.set_matrix(gl, UniformTypes::ProjMatrix, &camera.projection);
            gl.enable(glow::CULL_FACE);
            gl.enable(glow::DEPTH_TEST);
            gl.disable(glow::BLEND);
            gl.bind_vertex_array(Some(self.vertex_array));
            // Floating origin: the CPU sends each chunk's offset from the player's chunk
            // (exact in f32), so the shader does no rebase.
            let player = player_chunk_pos.as_vec();
            let mut draw_calls = 0;
            for (pos, chunk) in &self.gpu_chunks {
                let offset = ((pos.as_vec() - player) * CHUNK_SIZE as i32).as_vec3();
                gl.vertex_attrib_3_f32(OFFSET_ATTR, offset.x, offset.y, offset.z);
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(chunk.buffer));
                for side in SIDES {
                    let (first, count) = chunk.side_ranges[side as usize];
                    if count == 0 || !Self::is_visible(player_chunk_pos, *pos, side) {
                        continue;
                    }
                    gl.vertex_attrib_pointer_i32(DATA_ATTR, 1, glow::INT, 0, first * 4);
                    gl.draw_arrays_instanced(glow::TRIANGLE_STRIP, 0, 4, count);
                    draw_calls += 1;
                }
            }
            self.draw_calls.set(draw_calls);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_vertex_array(None);
        }
    }
}

fn compile_shader(gl: &glow::Context) -> Result<Rc<ShaderProgram>, String> {
    debug_assert!(
        CHUNK_SIZE == 16,
        "make sure to update shaders for new chunk size"
    );
    unsafe {
        let program = shader_def!(
            "chunk.vert",
            "chunk.frag",
            vec!(),
            vec!(
                (UniformTypes::ViewMatrix, "view"),
                (UniformTypes::ProjMatrix, "projection")
            )
        )
        .compile(gl)?;
        Ok(Rc::new(program))
    }
}
