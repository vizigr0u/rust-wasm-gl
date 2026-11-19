use glam::Mat4;
use std::{cell::Cell, collections::HashMap, rc::Rc};

use glow::{HasContext, WebBufferKey, WebVertexArrayKey};

use crate::{
    render::gl::{ShaderDef, ShaderProgram, TextureDef, UniformTypes},
    shader_def,
    world::{make_base_quad_data, ChunkDrawData, ChunkPos, CHUNK_SIZE},
};
use voxel_core::render::{build_draw_list, pack_sides, SideRanges};

#[derive(Debug)]
struct GraphicContext {
    program: Rc<ShaderProgram>,
}

/// One chunk's quads on the GPU: all 6 sides concatenated in one buffer.
#[derive(Debug)]
struct GpuChunk {
    buffer: WebBufferKey,
    /// (first quad, quad count) per `Side`.
    side_ranges: SideRanges,
}

const DATA_ATTR: u32 = 1;
const OFFSET_ATTR: u32 = 2;

#[derive(Debug)]
pub struct WorldRenderData {
    graphics: GraphicContext,
    vertex_array: WebVertexArrayKey,
    gpu_chunks: HashMap<ChunkPos, GpuChunk>,
    /// Draw calls issued by the last `render` (kept by both backends).
    draw_calls: Cell<u32>,
}

impl WorldRenderData {
    pub fn chunk_buffers(&self) -> usize {
        self.gpu_chunks.len()
    }

    pub fn draw_calls(&self) -> u32 {
        self.draw_calls.get()
    }

    /// Create or replace the GPU buffer of one chunk.
    pub fn upload_chunk(&mut self, gl: &glow::Context, pos: ChunkPos, data: &ChunkDrawData) {
        self.remove_chunk(gl, pos);
        if data.quad_count() == 0 {
            return;
        }
        let (all, side_ranges) = pack_sides(data);
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

    pub fn new(gl: &glow::Context) -> Result<Self, String> {
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
                graphics: GraphicContext { program },
                vertex_array,
                gpu_chunks: HashMap::new(),
                draw_calls: Cell::new(0),
            })
        }
    }

    pub fn render(
        &self,
        gl: &glow::Context,
        texture: &TextureDef,
        view: &Mat4,
        projection: &Mat4,
        player_chunk_pos: ChunkPos,
    ) {
        let graphics = &self.graphics;
        unsafe {
            let program = &graphics.program;
            program.gl_use(gl);
            gl.bind_texture(texture.0.into(), Some(texture.1));
            program.set_matrix(gl, UniformTypes::ViewMatrix, view);
            program.set_matrix(gl, UniformTypes::ProjMatrix, projection);
            gl.enable(glow::CULL_FACE);
            gl.enable(glow::DEPTH_TEST);
            gl.disable(glow::BLEND);
            gl.bind_vertex_array(Some(self.vertex_array));
            // Floating origin: the CPU sends each chunk's offset from the player's chunk
            // (exact in f32), so the shader does no rebase.
            let draws = build_draw_list(
                self.gpu_chunks.iter().map(|(p, c)| (*p, c.side_ranges)),
                player_chunk_pos,
            );
            let mut bound = None;
            for d in &draws {
                if bound != Some(d.chunk) {
                    let o = d.offset;
                    gl.vertex_attrib_3_f32(OFFSET_ATTR, o.x, o.y, o.z);
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.gpu_chunks[&d.chunk].buffer));
                    bound = Some(d.chunk);
                }
                gl.vertex_attrib_pointer_i32(DATA_ATTR, 1, glow::INT, 0, d.first as i32 * 4);
                gl.draw_arrays_instanced(glow::TRIANGLE_STRIP, 0, 4, d.count as i32);
            }
            let draw_calls = draws.len() as u32;
            self.draw_calls.set(draw_calls);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_vertex_array(None);
        }
    }
}

pub fn compile_shader(gl: &glow::Context) -> Result<Rc<ShaderProgram>, String> {
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
