use std::cell::RefCell;
use std::{collections::HashMap, convert::TryInto, rc::Rc};

use crate::shader_def;
use crate::utils::GlState;
use crate::{
    inputsystem::{HandleInputs, InputEventType, InputState},
    mesh::{Mesh, VertexAttrType},
    meshrenderer::MeshRenderer,
    shaders::UniformTypes,
};

use crate::shaders::ShaderDef;
use egui::{epaint::Primitive, Event, Key, TextureFilter, TextureId, TextureWrapMode};
use glow::{HasContext, WebTextureKey};
use log::{info, warn};
use web_sys::{KeyboardEvent, MouseEvent};

// struct DemoData {
//     pub name: String,
//     pub age: i32,
// }

// egui::CentralPanel::default().show(&ctx, |ui| {
//     ui.heading("My egui Application");
//     ui.horizontal(|ui| {
//         let name_label = ui.label("Your name: ");
//         ui.text_edit_singleline(&mut data.name)
//             .labelled_by(name_label.id);
//     });
//     ui.add(egui::Slider::new(&mut data.age, 0..=120).text("age"));
//     if ui.button("Increment").clicked() {
//         data.age += 1;
//     }
//     ui.label(format!(
//         "Hello '{name}', age {age}",
//         name = data.name,
//         age = data.age
//     ));

//     if ui.button("Click me").clicked() {
//         warn!("CLICK!");
//     }
// });

#[derive(Debug)]
pub struct EguiBackend {
    egui_ctx: egui::Context,
    egui_once: bool,
    textures: HashMap<TextureId, WebTextureKey>,
    mesh_renderer: MeshRenderer,
    current_events: Vec<Event>,
    size: (usize, usize),
}

impl EguiBackend {
    pub fn new(gl: &glow::Context) -> Self {
        let program = unsafe {
            shader_def!(
                "egui.vert",
                "egui.frag",
                vec!(
                    (VertexAttrType::Position, "position"),
                    (VertexAttrType::UVs, "uv"),
                    (VertexAttrType::Color, "color"),
                ),
                vec!((UniformTypes::ProjMatrix, "u_ortho"),)
            )
            .compile(gl)
        }
        .expect("Cant compile eguis shaders");
        let program = Rc::new(program);
        Self {
            egui_ctx: egui::Context::default(),
            egui_once: true,
            mesh_renderer: MeshRenderer::new(&program),
            textures: HashMap::new(),
            current_events: Vec::new(),
            size: (800, 600),
        }
    }

    pub fn render_ui<F>(&mut self, gl: &glow::Context, mut build_gui: F)
    where
        F: FnMut(&egui::Context),
    {
        let raw_input: egui::RawInput = self.gather_input();

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            build_gui(ctx);
        });
        self.handle_platform_output(full_output.platform_output);
        let clipped_primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        unsafe {
            let state = GlState::save(gl);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
            gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
            gl.blend_func_separate(
                // egui outputs colors with premultiplied alpha:
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
                // Less important, but this is technically the correct alpha blend function
                // when you want to make use of the framebuffer alpha (for screenshots, compositing, etc).
                glow::ONE_MINUS_DST_ALPHA,
                glow::ONE,
            );
            self.paint(gl, full_output.textures_delta, clipped_primitives);
            state.restore(gl);
        }
    }

    fn paint(
        &mut self,
        gl: &glow::Context,
        textures_delta: egui::TexturesDelta,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
    ) {
        // self.debug_paint(&textures_delta, &clipped_primitives);
        self.update_textures(gl, &textures_delta);
        for primitive in &clipped_primitives {
            match &primitive.primitive {
                Primitive::Mesh(p) => {
                    unsafe {
                        let tex = self.textures.get(&p.texture_id);
                        match tex {
                            Some(t) => gl.bind_texture(glow::TEXTURE_2D, Some(*t)),
                            None => {
                                warn!("Texture not found: {:?}", p.texture_id);
                                continue;
                            }
                        }
                    }
                    let mesh = Rc::new(Mesh::from(p));
                    self.mesh_renderer
                        .set_mesh(gl, mesh)
                        .expect("Can't set egui mesh");
                    let program = self.mesh_renderer.get_program();
                    program.gl_use(gl);
                    program.set_matrix(
                        gl,
                        UniformTypes::ProjMatrix,
                        &self.make_projection(800.0, 600.0),
                    );
                    self.mesh_renderer.render(gl);
                }
                _ => {
                    warn!("Cant render egui callback");
                }
            }
        }
    }

    fn make_projection(&self, width: f32, height: f32) -> glam::Mat4 {
        glam::Mat4::orthographic_rh_gl(0.0, width, height, 0.0, -1.0, 1.0)
    }

    fn update_textures(&mut self, gl: &glow::Context, textures_delta: &egui::TexturesDelta) {
        unsafe {
            for (id, img_delta) in &textures_delta.set {
                if !img_delta.is_whole() {
                    warn!("Partial update of texture {:?} not supported", id);
                    continue;
                }
                info!("loading texture: {:?}", id);
                let options = &img_delta.options;
                let key: WebTextureKey = gl.create_texture().expect("Can't create texture");
                gl.bind_texture(glow::TEXTURE_2D, Some(key));
                let wrap_mode = wrap_to_glow(options.wrap_mode);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap_mode as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap_mode as i32);
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    filter_to_glow(options.minification) as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    filter_to_glow(options.magnification) as i32,
                );
                let (internal_format, src_format) = (glow::SRGB8_ALPHA8, glow::RGBA); // if not SRGB : (glow::RGBA8, glow::RGBA);
                match &img_delta.image {
                    egui::ImageData::Color(image) => {
                        let data: &[u8] = bytemuck::cast_slice(image.pixels.as_ref());
                        gl.tex_image_2d(
                            glow::TEXTURE_2D,
                            0,
                            internal_format as _,
                            image.width() as _,
                            image.height() as _,
                            0,
                            src_format,
                            glow::UNSIGNED_BYTE,
                            Some(data),
                        );
                        self.textures.insert(*id, key);
                    }
                    egui::ImageData::Font(image) => {
                        gl.tex_image_2d(
                            glow::TEXTURE_2D,
                            0,
                            internal_format as _,
                            image.width() as _,
                            image.height() as _,
                            0,
                            src_format,
                            glow::UNSIGNED_BYTE,
                            Some(
                                &image
                                    .srgba_pixels(None)
                                    .flat_map(|p| p.to_array())
                                    .collect::<Vec<_>>(),
                            ),
                        );
                    }
                };
                self.textures.insert(*id, key);
            }
        }
    }

    fn debug_paint(
        &mut self,
        textures_delta: &egui::TexturesDelta,
        clipped_primitives: &Vec<egui::ClippedPrimitive>,
    ) {
        if self.egui_once {
            for (id, img_delta) in &textures_delta.set {
                info!("{id:?}, {:?}", img_delta.options);
            }
            for p in clipped_primitives {
                info!("prim: {:?}, {:?}", p.clip_rect, p.primitive);
            }
            self.egui_once = false;
        }
    }

    fn handle_platform_output(&self, _platform_output: egui::PlatformOutput) {}

    fn gather_input(&self) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect {
                min: egui::pos2(0.0, 0.0),
                max: egui::pos2(self.size.0 as f32, self.size.1 as f32),
            }),
            events: self.current_events.clone(),
            ..Default::default()
        }
    }

    fn set_events(&mut self, events: &Vec<InputEventType>) {
        self.current_events.clear();
        for e in events.iter().map(|e| e.try_into()) {
            match e {
                Ok(e) => {
                    self.current_events.push(e);
                }
                Err(e) => match e {
                    ConversionError::IgnoredKey(_) => {}
                    _ => warn!("Could not convert event: {e:?}"),
                },
            }
        }
    }
}

impl HandleInputs for EguiBackend {
    fn handle_inputs(&mut self, inputs: &InputState) {
        self.set_events(inputs.get_events());
    }
}

#[derive(Debug)]
pub enum ConversionError {
    KeyNotFound(String),
    UnknownButton(i16),
    IgnoredKey(String),
    Unhandled,
}

impl TryInto<Event> for &InputEventType {
    type Error = ConversionError;

    fn try_into(self) -> Result<Event, Self::Error> {
        let event = match self {
            InputEventType::KeyDown(event) => Event::Key {
                key: try_parse_key(event.key())?,
                pressed: true,
                modifiers: get_key_modifiers(&event),
                physical_key: None,
                repeat: false,
            },
            InputEventType::KeyUp(event) => Event::Key {
                key: try_parse_key(event.key())?,
                pressed: false,
                modifiers: get_key_modifiers(&event),
                physical_key: None,
                repeat: false,
            },
            InputEventType::MouseMove(event) => Event::PointerMoved(mouse_event_to_pos2(&event)),
            InputEventType::MouseDown(event) => Event::PointerButton {
                pos: mouse_event_to_pos2(&event),
                button: try_parse_mouse_button(event.button())?,
                pressed: true,
                modifiers: get_mouse_modifiers(&event),
            },
            InputEventType::MouseUp(event) => Event::PointerButton {
                pos: mouse_event_to_pos2(&event),
                button: try_parse_mouse_button(event.button())?,
                pressed: false,
                modifiers: get_mouse_modifiers(&event),
            },
            _ => Err(ConversionError::Unhandled)?,
        };
        Ok(event)
    }
}

fn get_key_modifiers(event: &KeyboardEvent) -> egui::Modifiers {
    let mut modifiers = egui::Modifiers::default();
    modifiers.shift = event.get_modifier_state("Shift");
    modifiers.ctrl = event.get_modifier_state("Control");
    modifiers.alt = event.get_modifier_state("Alt");
    modifiers.command = event.get_modifier_state("Meta");

    modifiers
}

fn get_mouse_modifiers(event: &MouseEvent) -> egui::Modifiers {
    let mut modifiers = egui::Modifiers::default();
    modifiers.shift = event.get_modifier_state("Shift");
    modifiers.ctrl = event.get_modifier_state("Control");
    modifiers.alt = event.get_modifier_state("Alt");
    modifiers.command = event.get_modifier_state("Meta");

    modifiers
}

fn try_parse_mouse_button(button: i16) -> Result<egui::PointerButton, ConversionError> {
    match button {
        0 => Ok(egui::PointerButton::Primary),
        1 => Ok(egui::PointerButton::Middle),
        2 => Ok(egui::PointerButton::Secondary),
        n => Err(ConversionError::UnknownButton(n))?,
    }
}

fn try_parse_key(key: String) -> Result<egui::Key, ConversionError> {
    if key == "Shift" || key == "Control" || key == "Alt" || key == "Meta" {
        Err(ConversionError::IgnoredKey(key))
    } else {
        Key::from_name(key.as_str()).ok_or_else(|| ConversionError::KeyNotFound(key))
    }
}

fn mouse_event_to_pos2(event: &MouseEvent) -> egui::Pos2 {
    egui::pos2(event.client_x() as f32, event.client_y() as f32)
}

/* convert a TextureFilter to a glow filter */
fn filter_to_glow(filter: TextureFilter) -> u32 {
    match filter {
        TextureFilter::Linear => glow::LINEAR,
        TextureFilter::Nearest => glow::NEAREST,
    }
}

/* convert a TextureWrapMode to a glow filter */
fn wrap_to_glow(wrap: TextureWrapMode) -> u32 {
    match wrap {
        TextureWrapMode::ClampToEdge => glow::CLAMP_TO_EDGE,
        TextureWrapMode::Repeat => glow::REPEAT,
        TextureWrapMode::MirroredRepeat => glow::MIRRORED_REPEAT,
    }
}

impl From<&egui::Mesh> for Mesh {
    fn from(mesh: &egui::Mesh) -> Self {
        let mut data = Vec::<f32>::with_capacity(mesh.vertices.len() * 8);
        for v in &mesh.vertices {
            data.push(v.pos.x);
            data.push(v.pos.y);
            data.push(v.uv.x);
            data.push(v.uv.y);
            data.push(v.color.r() as f32 / 255.0);
            data.push(v.color.g() as f32 / 255.0);
            data.push(v.color.b() as f32 / 255.0);
            data.push(v.color.a() as f32 / 255.0);
        }
        Mesh {
            data,
            primitive_type: glow::TRIANGLES,
            layout: vec![
                (VertexAttrType::Position, 2),
                (VertexAttrType::UVs, 2),
                (VertexAttrType::Color, 4),
            ],
            indices: Some(mesh.indices.clone()),
        }
    }
}
