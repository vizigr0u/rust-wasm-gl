use std::{collections::HashMap, convert::TryInto, rc::Rc};

use crate::core::{HandleInputs, InputEventType, InputState};
use crate::shader_def;
use crate::utils::GlRenderFlags;
use crate::{core::Time, graphics::MeshRenderer};

use crate::graphics::{Mesh, ShaderDef, UniformTypes, VertexAttrType};
use egui::{epaint::Primitive, Event, Key, TextureFilter, TextureId, TextureWrapMode};
use glow::{HasContext, WebTextureKey};
use log::warn;
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
    // egui_once: bool,
    textures: HashMap<TextureId, WebTextureKey>,
    mesh_renderer: MeshRenderer,
    current_events: Vec<Event>,
    size: (usize, usize),
    time: Time,
}

const RENDER_FLAGS: [GlRenderFlags; 1] = [GlRenderFlags::Blend];

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
            // egui_once: true,
            mesh_renderer: MeshRenderer::with_render_flags(&RENDER_FLAGS, &program),
            textures: HashMap::new(),
            current_events: Vec::new(),
            size: (800, 600),
            time: Time::default(),
        }
    }

    pub fn render_ui<F>(&mut self, gl: &glow::Context, mut build_gui: F)
    where
        F: FnMut(&egui::Context),
    {
        self.update_size();
        let raw_input: egui::RawInput = self.gather_input();

        let full_output = self.egui_ctx.run_pass(raw_input, |ctx| {
            build_gui(ctx);
        });
        self.handle_platform_output(full_output.platform_output);
        let clipped_primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        unsafe {
            self.paint(gl, full_output.textures_delta, clipped_primitives);
        }
    }

    /// Read the real drawing-buffer size from the canvas (egui points == pixels here).
    fn update_size(&mut self) {
        if let Ok(canvas) = crate::utils::get_canvas() {
            self.size = (canvas.width() as usize, canvas.height() as usize);
        }
    }

    fn paint(
        &mut self,
        gl: &glow::Context,
        mut textures_delta: egui::TexturesDelta,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
    ) {
        // self.debug_paint(&textures_delta, &clipped_primitives);
        self.update_textures(gl, &textures_delta);
        for id in &textures_delta.free {
            if let Some(key) = self.textures.remove(id) {
                unsafe { gl.delete_texture(key) };
            }
        }
        textures_delta.clear();
        let (width, height) = (self.size.0 as f32, self.size.1 as f32);
        unsafe { gl.enable(glow::SCISSOR_TEST) };
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
                    // GL scissor origin is bottom-left; egui's is top-left.
                    let clip = primitive.clip_rect;
                    let x0 = clip.min.x.clamp(0.0, width).round();
                    let x1 = clip.max.x.clamp(0.0, width).round();
                    let y0 = clip.min.y.clamp(0.0, height).round();
                    let y1 = clip.max.y.clamp(0.0, height).round();
                    unsafe {
                        gl.scissor(
                            x0 as i32,
                            (height - y1) as i32,
                            (x1 - x0).max(0.0) as i32,
                            (y1 - y0).max(0.0) as i32,
                        );
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
                        &self.make_projection(width, height),
                    );
                    unsafe {
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
                    }
                    self.mesh_renderer.render(gl);
                }
                _ => {
                    warn!("Cant render egui callback");
                }
            }
        }
        unsafe { gl.disable(glow::SCISSOR_TEST) };
    }

    fn make_projection(&self, width: f32, height: f32) -> glam::Mat4 {
        glam::camera::rh::proj::opengl::orthographic(0.0, width, height, 0.0, -1.0, 1.0)
    }

    fn update_textures(&mut self, gl: &glow::Context, textures_delta: &egui::TexturesDelta) {
        unsafe {
            for (id, deltas) in &textures_delta.set {
                for img_delta in deltas {
                    if let Some(pos) = img_delta.pos {
                        let Some(key) = self.textures.get(id).copied() else {
                            warn!("Partial update of unknown texture {:?}", id);
                            continue;
                        };
                        gl.bind_texture(glow::TEXTURE_2D, Some(key));
                        match &img_delta.image {
                            egui::ImageData::Color(image) => {
                                let data: &[u8] = bytemuck::cast_slice(image.pixels.as_ref());
                                gl.tex_sub_image_2d(
                                    glow::TEXTURE_2D,
                                    0,
                                    pos[0] as _,
                                    pos[1] as _,
                                    image.width() as _,
                                    image.height() as _,
                                    glow::RGBA,
                                    glow::UNSIGNED_BYTE,
                                    glow::PixelUnpackData::Slice(Some(data)),
                                );
                            }
                        }
                        continue;
                    }
                    let options = &img_delta.options;
                    if let Some(old) = self.textures.remove(id) {
                        gl.delete_texture(old);
                    }
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
                                glow::PixelUnpackData::Slice(Some(data)),
                            );
                        }
                    };
                    self.textures.insert(*id, key);
                }
            }
        }
    }

    // fn debug_paint(
    //     &mut self,
    //     textures_delta: &egui::TexturesDelta,
    //     clipped_primitives: &Vec<egui::ClippedPrimitive>,
    // ) {
    //     if self.egui_once {
    //         for (id, img_delta) in &textures_delta.set {
    //             info!("{id:?}, {:?}", img_delta.options);
    //         }
    //         for p in clipped_primitives {
    //             info!("prim: {:?}, {:?}", p.clip_rect, p.primitive);
    //         }
    //         self.egui_once = false;
    //     }
    // }

    fn handle_platform_output(&self, _platform_output: egui::PlatformOutput) {}

    fn gather_input(&self) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect {
                min: egui::pos2(0.0, 0.0),
                max: egui::pos2(self.size.0 as f32, self.size.1 as f32),
            }),
            events: self.current_events.clone(),
            // time: Some(self.time.time()),
            // predicted_dt: self.time.delta_time() as _,
            ..Default::default()
        }
    }

    fn set_events(&mut self, events: &Vec<InputEventType>) {
        self.current_events.clear();
        for ev in events.iter() {
            let e: Result<Event, ConversionError> = ev.try_into();
            match e {
                Ok(e) => {
                    self.current_events.push(e);
                    // Printable keys also produce text for text edits.
                    if let Some(text) = text_for_event(ev) {
                        self.current_events.push(Event::Text(text));
                    }
                }
                Err(e) => match e {
                    ConversionError::IgnoredKey(_) => {}
                    _ => warn!("Could not convert event: {e:?}"),
                },
            }
        }
    }

    pub fn clear_events(&mut self) {
        self.current_events.clear();
    }

    pub fn set_time(&mut self, time: &Time) {
        self.time = time.clone();
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
                repeat: event.repeat(),
            },
            InputEventType::KeyUp(event) => Event::Key {
                key: try_parse_key(event.key())?,
                pressed: false,
                modifiers: get_key_modifiers(&event),
                physical_key: None,
                repeat: event.repeat(),
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
            InputEventType::MouseWheel(event) => {
                let unit = match event.delta_mode() {
                    1 => egui::MouseWheelUnit::Line,
                    2 => egui::MouseWheelUnit::Page,
                    _ => egui::MouseWheelUnit::Point,
                };
                Event::MouseWheel {
                    unit,
                    // Web deltaY is positive when scrolling down; egui wants the opposite.
                    delta: egui::vec2(-event.delta_x() as f32, -event.delta_y() as f32),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: get_mouse_modifiers(&event),
                }
            }
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

/// Text typed by a KeyDown event (printable single characters, no ctrl/meta chords).
fn text_for_event(event: &InputEventType) -> Option<String> {
    let InputEventType::KeyDown(e) = event else {
        return None;
    };
    if e.ctrl_key() || e.meta_key() {
        return None;
    }
    let key = e.key();
    (key.chars().count() == 1).then_some(key)
}

/// Canvas-relative position in drawing-buffer pixels (accounts for CSS scaling).
fn mouse_event_to_pos2(event: &MouseEvent) -> egui::Pos2 {
    let (cx, cy) = (event.client_x() as f32, event.client_y() as f32);
    let Ok(canvas) = crate::utils::get_canvas() else {
        return egui::pos2(cx, cy);
    };
    let rect = canvas.get_bounding_client_rect();
    let sx = if rect.width() > 0.0 { canvas.width() as f32 / rect.width() as f32 } else { 1.0 };
    let sy = if rect.height() > 0.0 { canvas.height() as f32 / rect.height() as f32 } else { 1.0 };
    egui::pos2((cx - rect.left() as f32) * sx, (cy - rect.top() as f32) * sy)
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
