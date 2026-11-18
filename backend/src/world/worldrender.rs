use std::{collections::HashMap, rc::Rc};

use glam::{IVec3, Vec3};
use glow::{HasContext, WebBufferKey, WebVertexArrayKey};
use log::info;

use crate::{
    graphics::{Camera, ShaderDef, ShaderProgram, Side, TextureType, UniformTypes, VertexAttrType},
    shader_def,
    utils::make_quad,
    world::{Chunk, CHUNK_SIZE},
};

use super::{make_base_quad_data, BlockPos, ChunkPos};


#[derive(Debug)]
struct GraphicContext {
    program: Rc<ShaderProgram>,
    texture: Rc<(TextureType, glow::WebTextureKey)>,
}

#[derive(Debug, Clone)]
pub struct ChunkDrawData {
    pub vertex_data: [Vec<i32>; 6],
}

#[derive(Debug)]
pub struct WorldRenderData {
    graphics: Option<GraphicContext>,
    vertex_array: Option<WebVertexArrayKey>,
    num_quads_to_draw: i32,
    vertex_data_buffer: Option<WebBufferKey>,
    location_data_buffer: Option<WebBufferKey>,
    vertex_data_cache: Vec<i32>,
    location_data_cache: Vec<Vec3>,
    peak_vertex_len: usize,
    peak_location_len: usize,
}

impl ChunkDrawData {
    pub fn quad_count(&self) -> usize {
        self.vertex_data.iter().map(Vec::len).sum()
    }

    pub fn load(chunk: &Chunk) -> Result<Self, String> {
        let vertex_data = chunk.to_vertex_data();
        Ok(Self { vertex_data })
    }
}

impl WorldRenderData {
    pub fn new() -> Self {
        Self {
            graphics: None,
            vertex_array: None,
            num_quads_to_draw: 0,
            vertex_data_buffer: None,
            location_data_buffer: None,
            vertex_data_cache: Vec::new(),
            location_data_cache: Vec::new(),
            peak_vertex_len: 0,
            peak_location_len: 0,
        }
    }

    /// Peak (vertex i32 count, location Vec3 count) seen in a gather.
    pub fn peak_cache_len(&self) -> (usize, usize) {
        (self.peak_vertex_len, self.peak_location_len)
    }

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

    fn add_if_visible(
        &mut self,
        data: &ChunkDrawData,
        side: Side,
        player_chunk_pos: ChunkPos,
        data_pos: ChunkPos,
    ) -> usize {
        if !Self::is_visible(player_chunk_pos, data_pos, side) {
            return 0;
        }
        let data = &data.vertex_data[side as usize];
        self.vertex_data_cache.extend_from_slice(data);
        let block_pos_f3 = data_pos.get_center_block_pos().as_vec3();
        self.location_data_cache
            .extend(std::iter::repeat(block_pos_f3).take(data.len()));
        data.len()
    }

    pub fn gather_draw_data(
        &mut self,
        gl: &glow::Context,
        player_chunk_pos: ChunkPos,
        loaded_chunks: &HashMap<ChunkPos, Option<ChunkDrawData>>,
    ) {
        self.num_quads_to_draw = 0;
        self.vertex_data_cache.clear();
        self.location_data_cache.clear();
        for (chunk_pos, draw_data) in loaded_chunks {
            if let Some(draw_data) = draw_data {
                let mut quad_count: usize = 0;
                quad_count +=
                    self.add_if_visible(&draw_data, Side::Front, player_chunk_pos, *chunk_pos);
                quad_count +=
                    self.add_if_visible(&draw_data, Side::Back, player_chunk_pos, *chunk_pos);
                quad_count +=
                    self.add_if_visible(&draw_data, Side::Left, player_chunk_pos, *chunk_pos);
                quad_count +=
                    self.add_if_visible(&draw_data, Side::Right, player_chunk_pos, *chunk_pos);
                quad_count +=
                    self.add_if_visible(&draw_data, Side::Top, player_chunk_pos, *chunk_pos);
                quad_count +=
                    self.add_if_visible(&draw_data, Side::Bottom, player_chunk_pos, *chunk_pos);
                self.num_quads_to_draw += quad_count as i32;
            }
        }

        self.peak_vertex_len = self.peak_vertex_len.max(self.vertex_data_cache.len());
        self.peak_location_len = self.peak_location_len.max(self.location_data_cache.len());

        // debug logs
        if true {
            let num_verts = self.vertex_data_cache.len();
            let num_locs = self.location_data_cache.len();
            info!(
                "Uploaded {} vertices and {} locations for {} quads",
                num_verts,
                num_locs / 3,
                num_verts
            );
            if num_verts >= 4 && num_locs >= 4 {
                info!(
                    "verts: {:?}, {:?}, ... {:?}, {:?} - locs: {:?}, {:?}, {:?}, ... {:?}, {:?}, {:?}",
                    self.vertex_data_cache[0],
                    self.vertex_data_cache[1],
                    self.vertex_data_cache[num_verts - 2],
                    self.vertex_data_cache[num_verts - 1],
                    self.location_data_cache[0],
                    self.location_data_cache[1],
                    self.location_data_cache[2],
                    self.location_data_cache[num_locs - 3],
                    self.location_data_cache[num_locs - 2],
                    self.location_data_cache[num_locs - 1],
                );
            }
        }
        unsafe {
            {
                // upload vertex data
                gl.bind_buffer(glow::ARRAY_BUFFER, self.vertex_data_buffer);
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    &self.vertex_data_cache.align_to::<u8>().1,
                    glow::STATIC_DRAW,
                );
            }

            {
                // upload location data
                gl.bind_buffer(glow::ARRAY_BUFFER, self.location_data_buffer);
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    &self.location_data_cache.align_to::<u8>().1,
                    glow::STATIC_DRAW,
                );
            }
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }

    pub fn setup_graphics(
        &mut self,
        gl: &glow::Context,
        texture: Rc<(TextureType, glow::WebTextureKey)>,
    ) -> Result<(), String> {
        let program = compile_shader(gl)?;

        unsafe {
            if true {
                program.gl_use(gl);

                let vao = Some(gl.create_vertex_array()?);
                gl.bind_vertex_array(vao);
                self.vertex_array = vao;

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
            }

            if true {
                // setup vertex data attributes - 1 per quad
                self.vertex_data_buffer = Some(gl.create_buffer()?);
                gl.bind_buffer(glow::ARRAY_BUFFER, self.vertex_data_buffer);

                let data_attr_pos = 1;
                gl.enable_vertex_attrib_array(data_attr_pos);
                gl.vertex_attrib_pointer_i32(data_attr_pos, 1, glow::INT, 0, 0);
                gl.vertex_attrib_divisor(data_attr_pos, 1);
            }
            if true {
                // setup vertex location attributes - 1 per quad
                self.location_data_buffer = Some(gl.create_buffer()?);
                gl.bind_buffer(glow::ARRAY_BUFFER, self.location_data_buffer);

                let location_attr_pos = 2;
                gl.enable_vertex_attrib_array(location_attr_pos);
                gl.vertex_attrib_pointer_f32(location_attr_pos, 3, glow::FLOAT, false, 0, 0);
                gl.vertex_attrib_divisor(location_attr_pos, 1);
            }
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_vertex_array(None);
        }
        self.graphics = Some(GraphicContext { program, texture });
        Ok(())
    }

    pub fn render(&self, gl: &glow::Context, camera: &Camera, player_chunk_pos: ChunkPos) {
        if let Some(graphics) = &self.graphics {
            unsafe {
                let program = &graphics.program;
                program.gl_use(gl);
                gl.bind_texture(graphics.texture.0.into(), Some(graphics.texture.1));
                program.set_current_player_chunk(gl, player_chunk_pos);
                program.set_matrix(gl, UniformTypes::ViewMatrix, &camera.look_at);
                program.set_matrix(gl, UniformTypes::ProjMatrix, &camera.projection);
                gl.enable(glow::CULL_FACE);
                gl.enable(glow::DEPTH_TEST);
                gl.disable(glow::BLEND);
                gl.bind_vertex_array(self.vertex_array);
                {
                    let num_quads = self.num_quads_to_draw as i32;
                    gl.draw_arrays_instanced(glow::TRIANGLE_STRIP, 0, 4, num_quads);
                }
                gl.bind_vertex_array(None);
            }
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
                (UniformTypes::ProjMatrix, "projection"),
                (UniformTypes::PlayerChunkPos, "player_chunk_pos")
            )
        )
        .compile(gl)?;
        Ok(Rc::new(program))
    }
}
