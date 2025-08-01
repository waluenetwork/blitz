use anyrender_vello::wgpu_context::DeviceHandle;
use anyrender_vello::{CustomPaintCtx, CustomPaintSource, TextureHandle};
use egui::{Context, RawInput, Event, PointerButton, Key, Modifiers};
use egui_wgpu;
use std::sync::mpsc::{channel, Receiver, Sender};
use wgpu::Instance;
use blitz_traits::events::{BlitzKeyEvent, BlitzImeEvent};
use keyboard_types;

pub struct EguiPaintSource {
    egui_ctx: Context,
    state: EguiRendererState,
    tx: Sender<RawInput>,
    rx: Receiver<RawInput>,
    ui_fn: Option<Box<dyn Fn(&Context) + Send + 'static>>,
}


pub enum EguiRendererState {
    Active {
        device: wgpu::Device,
        queue: wgpu::Queue,
        egui_renderer: egui_wgpu::Renderer,
        texture: Option<wgpu::Texture>,
        texture_handle: Option<TextureHandle>,
    },
    Suspended,
}

impl EguiPaintSource {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            egui_ctx: Context::default(),
            state: EguiRendererState::Suspended,
            tx,
            rx,
            ui_fn: None,
        }
    }

    pub fn with_ui<F>(ui_fn: F) -> Self
    where
        F: Fn(&Context) + Send + 'static,
    {
        let (tx, rx) = channel();
        Self {
            egui_ctx: Context::default(),
            state: EguiRendererState::Suspended,
            tx,
            rx,
            ui_fn: Some(Box::new(ui_fn)),
        }
    }

    pub fn sender(&self) -> Sender<RawInput> {
        self.tx.clone()
    }


    
    fn convert_event_to_raw_input(&self, x: f32, y: f32, event_type: &str, width: u32, height: u32) -> Option<RawInput> {
        let mut raw_input = RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(width as f32, height as f32),
        ));

        match event_type {
            "mousemove" => {
                raw_input.events.push(Event::PointerMoved(egui::Pos2::new(x, y)));
            }
            "mousedown" => {
                raw_input.events.push(Event::PointerButton {
                    pos: egui::Pos2::new(x, y),
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            "mouseup" => {
                raw_input.events.push(Event::PointerButton {
                    pos: egui::Pos2::new(x, y),
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            "click" => {
                let pos = egui::Pos2::new(x, y);
                raw_input.events.push(Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
                raw_input.events.push(Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            _ => return None,
        }

        Some(raw_input)
    }
    
    fn convert_key_event_to_raw_input(&self, key_event: &BlitzKeyEvent, width: u32, height: u32) -> Option<RawInput> {
        let mut raw_input = RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(width as f32, height as f32),
        ));

        let egui_key = match key_event.key {
            keyboard_types::Key::Character(ref s) => {
                if let Some(ch) = s.chars().next() {
                    match ch {
                        'a'..='z' | 'A'..='Z' => {
                            let key_char = ch.to_ascii_uppercase();
                            match key_char {
                                'A' => Key::A, 'B' => Key::B, 'C' => Key::C, 'D' => Key::D,
                                'E' => Key::E, 'F' => Key::F, 'G' => Key::G, 'H' => Key::H,
                                'I' => Key::I, 'J' => Key::J, 'K' => Key::K, 'L' => Key::L,
                                'M' => Key::M, 'N' => Key::N, 'O' => Key::O, 'P' => Key::P,
                                'Q' => Key::Q, 'R' => Key::R, 'S' => Key::S, 'T' => Key::T,
                                'U' => Key::U, 'V' => Key::V, 'W' => Key::W, 'X' => Key::X,
                                'Y' => Key::Y, 'Z' => Key::Z,
                                _ => return None,
                            }
                        }
                        '0'..='9' => {
                            match ch {
                                '0' => Key::Num0, '1' => Key::Num1, '2' => Key::Num2, '3' => Key::Num3,
                                '4' => Key::Num4, '5' => Key::Num5, '6' => Key::Num6, '7' => Key::Num7,
                                '8' => Key::Num8, '9' => Key::Num9,
                                _ => return None,
                            }
                        }
                        ' ' => Key::Space,
                        _ => return None,
                    }
                } else {
                    return None;
                }
            }
            keyboard_types::Key::Enter => Key::Enter,
            keyboard_types::Key::Tab => Key::Tab,
            keyboard_types::Key::Backspace => Key::Backspace,
            keyboard_types::Key::Delete => Key::Delete,
            keyboard_types::Key::ArrowLeft => Key::ArrowLeft,
            keyboard_types::Key::ArrowRight => Key::ArrowRight,
            keyboard_types::Key::ArrowUp => Key::ArrowUp,
            keyboard_types::Key::ArrowDown => Key::ArrowDown,
            keyboard_types::Key::Home => Key::Home,
            keyboard_types::Key::End => Key::End,
            keyboard_types::Key::Escape => Key::Escape,
            _ => return None,
        };

        let mut egui_modifiers = Modifiers::NONE;
        if key_event.modifiers.contains(keyboard_types::Modifiers::CONTROL) {
            egui_modifiers |= Modifiers::CTRL;
        }
        if key_event.modifiers.contains(keyboard_types::Modifiers::SHIFT) {
            egui_modifiers |= Modifiers::SHIFT;
        }
        if key_event.modifiers.contains(keyboard_types::Modifiers::ALT) {
            egui_modifiers |= Modifiers::ALT;
        }
        if key_event.modifiers.contains(keyboard_types::Modifiers::META) {
            egui_modifiers |= Modifiers::MAC_CMD;
        }

        let pressed = key_event.state.is_pressed();
        raw_input.events.push(Event::Key {
            key: egui_key,
            physical_key: None,
            pressed,
            repeat: key_event.is_auto_repeating,
            modifiers: egui_modifiers,
        });

        if pressed {
            if let Some(text) = &key_event.text {
                if !text.is_empty() {
                    raw_input.events.push(Event::Text(text.to_string()));
                }
            }
        }

        Some(raw_input)
    }
    
    fn convert_ime_event_to_raw_input(&self, ime_event: &BlitzImeEvent, width: u32, height: u32) -> Option<RawInput> {
        let mut raw_input = RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(width as f32, height as f32),
        ));

        match ime_event {
            BlitzImeEvent::Preedit(text, _) => {
                raw_input.events.push(Event::Text(text.clone()));
            }
            BlitzImeEvent::Commit(text) => {
                raw_input.events.push(Event::Text(text.clone()));
            }
            BlitzImeEvent::Enabled => {
            }
            BlitzImeEvent::Disabled => {
            }
        }

        Some(raw_input)
    }

    fn create_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("egui_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }


}

impl CustomPaintSource for EguiPaintSource {
    fn wants_events(&self) -> bool {
        true
    }
    
    fn handle_event(&mut self, x: f32, y: f32, event_type: &str) -> bool {
        let width = 400;
        let height = 300;
        
        if let Some(raw_input) = self.convert_event_to_raw_input(x, y, event_type, width, height) {
            if let Err(_) = self.tx.send(raw_input) {
                return false;
            }
            return true;
        }
        false
    }
    
    fn handle_key_event(&mut self, key_event: &dyn std::any::Any) -> bool {
        if let Some(key_event) = key_event.downcast_ref::<BlitzKeyEvent>() {
            let width = 400;
            let height = 300;
            
            if let Some(raw_input) = self.convert_key_event_to_raw_input(key_event, width, height) {
                if let Err(_) = self.tx.send(raw_input) {
                    return false;
                }
                return true;
            }
        }
        false
    }
    
    fn handle_ime_event(&mut self, ime_event: &dyn std::any::Any) -> bool {
        if let Some(ime_event) = ime_event.downcast_ref::<BlitzImeEvent>() {
            let width = 400;
            let height = 300;
            
            if let Some(raw_input) = self.convert_ime_event_to_raw_input(ime_event, width, height) {
                if let Err(_) = self.tx.send(raw_input) {
                    return false;
                }
                return true;
            }
        }
        false
    }
    fn resume(&mut self, _instance: &Instance, device_handle: &DeviceHandle) {
        println!("DEBUG: EguiPaintSource::resume called");
        
        let egui_renderer = egui_wgpu::Renderer::new(
            &device_handle.device,
            wgpu::TextureFormat::Rgba8Unorm,
            None,
            1,
            false,
        );
        
        self.state = EguiRendererState::Active {
            device: device_handle.device.clone(),
            queue: device_handle.queue.clone(),
            egui_renderer,
            texture: None,
            texture_handle: None,
        };
        
        println!("DEBUG: EguiPaintSource::resume completed with egui-wgpu renderer");
    }

    fn suspend(&mut self) {
        println!("DEBUG: EguiPaintSource::suspend called");
        self.state = EguiRendererState::Suspended;
    }

    fn render(
        &mut self,
        mut ctx: CustomPaintCtx<'_>,
        width: u32,
        height: u32,
        _scale: f64,
    ) -> Option<TextureHandle> {
        println!("DEBUG: EguiPaintSource::render called with dimensions: {}x{}", width, height);
        println!("DEBUG: EguiPaintSource state: {:?}", match self.state {
            EguiRendererState::Active { .. } => "Active",
            EguiRendererState::Suspended => "Suspended",
        });
        
        if width == 0 || height == 0 {
            println!("DEBUG: EguiPaintSource::render early return - invalid dimensions");
            return None;
        }

        let &mut EguiRendererState::Active {
            ref device,
            ref queue,
            ref mut egui_renderer,
            ref mut texture,
            ref mut texture_handle,
        } = &mut self.state
        else {
            println!("DEBUG: EguiPaintSource::render - state not active");
            return None;
        };

        println!("DEBUG: EguiPaintSource::render - state is active, proceeding with WGPU renderer");

        if let Some(tex) = texture {
            if tex.width() != width || tex.height() != height {
                if let Some(handle) = texture_handle.take() {
                    ctx.unregister_texture(handle);
                }
                *texture = None;
            }
        }

        if texture.is_none() {
            let new_texture = Self::create_texture(device, width, height);
            let handle = ctx.register_texture(new_texture.clone());
            *texture = Some(new_texture);
            *texture_handle = Some(handle);
            println!("DEBUG: Created new texture {}x{}", width, height);
        }

        let texture_ref = texture.as_ref().unwrap();
        let handle = texture_handle.unwrap();

        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(width as f32, height as f32),
        ));

        while let Ok(input) = self.rx.try_recv() {
            raw_input.events.extend(input.events.clone());
            if input.screen_rect.is_some() {
                raw_input.screen_rect = input.screen_rect;
            }
        }
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            if let Some(ref ui_fn) = self.ui_fn {
                ui_fn(ctx);
            } else {
                egui::Window::new("Egui Demo").show(ctx, |ui| {
                    ui.label("Hello from egui in Blitz!");
                    if ui.button("Click me").clicked() {
                        
                    }
                });
            }
        });


        let pixels_per_point = 1.0; // TODO: use actual scale
        let _shapes_count = full_output.shapes.len();
        let clipped_primitives = self.egui_ctx.tessellate(full_output.shapes, pixels_per_point);
        

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui_encoder"),
        });

        let view = texture_ref.create_view(&wgpu::TextureViewDescriptor::default());
        
        for (id, image_delta) in &full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: 1.0,
        };

        egui_renderer.update_buffers(device, queue, &mut encoder, &clipped_primitives, &screen_descriptor);

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            egui_renderer.render(&mut render_pass.forget_lifetime(), &clipped_primitives, &screen_descriptor);
        } // render_pass is dropped here, releasing the borrow on encoder

        for id in &full_output.textures_delta.free {
            egui_renderer.free_texture(id);
        }

        queue.submit(Some(encoder.finish()));

        Some(handle)
    }
}

impl Default for EguiPaintSource {
    fn default() -> Self {
        Self::new()
    }
}
