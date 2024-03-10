use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec3;
use web_sys::WebGl2RenderingContext;

use crate::include_shader;
use crate::shader_def;
use crate::shaders::ShaderDef;

pub struct Game {
    tris: Vec<Tri>,
    context: Rc<RefCell<WebGl2RenderingContext>>,
}

impl Game {
    pub fn new(context: Rc<RefCell<WebGl2RenderingContext>>) -> Self {
        let mut tris = Vec::<Tri>::new();
        tris.push(Tri::new(
            Vec3 {
                x: -0.7,
                y: -0.7,
                z: 0.0,
            },
            1.4,
        ));
        tris.push(Tri::new(
            Vec3 {
                x: -0.7,
                y: 0.0,
                z: 0.0,
            },
            0.3,
        ));
        Game { tris, context }
    }

    pub fn init(&self) -> Result<(), String> {
        let shader_def = shader_def!("white.vert", "white.frag");
        let context = self.context.borrow();
        let program = shader_def.compile(&self.context.borrow())?;
        self.context.borrow().use_program(Some(&program));

        let position_attribute_location = context.get_attrib_location(&program, "position");
        let color_attribute_location = context.get_attrib_location(&program, "vertexColor");
        let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
        context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

        let vao = context
            .create_vertex_array()
            .ok_or("Could not create vertex array object")?;
        context.bind_vertex_array(Some(&vao));

        context.vertex_attrib_pointer_with_i32(
            position_attribute_location as u32,
            3,
            WebGl2RenderingContext::FLOAT,
            false,
            6 * 4,
            0,
        );
        context.enable_vertex_attrib_array(position_attribute_location as u32);

        context.vertex_attrib_pointer_with_i32(
            color_attribute_location as u32,
            3,
            WebGl2RenderingContext::FLOAT,
            false,
            6 * 4,
            3 * 4,
        );
        context.enable_vertex_attrib_array(color_attribute_location as u32);

        Ok(())
    }

    pub fn draw(&self, _time: f64) {
        self.context.borrow().clear_color(0.0, 0.0, 0.0, 1.0);
        self.context
            .borrow()
            .clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

        // self.tri.draw(&self.context.borrow());
        for tri in &self.tris {
            tri.draw(&self.context.borrow());
        }
    }
}

pub struct Tri {
    pub pos: Vec3,
    pub size: f32,

    buffer: [f32; 18],
}

impl Tri {
    pub fn new(pos: Vec3, size: f32) -> Self {
        Tri {
            pos,
            size,
            buffer: [
                // Vertex 1
                pos.x,
                pos.y,
                0.0, // Position
                1.0,
                0.0,
                0.0, // Color (Red)
                // Vertex 2
                pos.x + size,
                pos.y,
                0.0, // Position
                0.0,
                1.0,
                0.0, // Color (Green)
                // Vertex 3
                pos.x + size * 0.5,
                pos.y + size,
                0.0, // Position
                0.0,
                0.0,
                1.0, // Color (Blue)
            ],
        }
    }

    pub fn draw(&self, context: &WebGl2RenderingContext) {
        // Note that `Float32Array::view` is somewhat dangerous (hence the
        // `unsafe`!). This is creating a raw view into our module's
        // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
        // (aka do a memory allocation in Rust) it'll cause the buffer to change,
        // causing the `Float32Array` to be invalid.
        //
        // As a result, after `Float32Array::view` we have to be very careful not to
        // do any memory allocations before it's dropped.
        unsafe {
            let positions_array_buf_view = js_sys::Float32Array::view(&self.buffer);

            context.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &positions_array_buf_view,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }

        let vert_count = (self.buffer.len() / 6) as i32;
        context.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, vert_count);
    }
}

// pub struct Cube {
//     pub pos: Vec3,
// }
