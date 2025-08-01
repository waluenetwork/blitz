use anyrender_vello::wgpu_context::DeviceHandle;
use anyrender_vello::{CustomPaintCtx, CustomPaintSource, TextureHandle};
use egui::{Context, RawInput};
use egui_wgpu;
use std::sync::mpsc::{channel, Receiver, Sender};
use wgpu::Instance;

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


    fn process_input(&mut self) {
        while let Ok(input) = self.rx.try_recv() {
            self.egui_ctx.begin_pass(input);
        }
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

        while let Ok(input) = self.rx.try_recv() {
            self.egui_ctx.begin_pass(input);
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

        println!("DEBUG: Running egui context with screen size: {}x{}", width, height);
        
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            if let Some(ref ui_fn) = self.ui_fn {
                ui_fn(ctx);
            } else {
                egui::Window::new("Egui Demo").show(ctx, |ui| {
                    ui.label("Hello from egui in Blitz!");
                    if ui.button("Click me").clicked() {
                        println!("Button clicked in egui!");
                    }
                });
            }
        });

        let shapes_count = full_output.shapes.len();
        println!("DEBUG: Egui generated {} shapes", shapes_count);

        let pixels_per_point = 1.0; // TODO: use actual scale
        let clipped_primitives = self.egui_ctx.tessellate(full_output.shapes, pixels_per_point);
        println!("DEBUG: Tessellated into {} primitives", clipped_primitives.len());

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
        println!("DEBUG: EguiPaintSource::render completed successfully with WGPU");

        Some(handle)
    }
}

impl Default for EguiPaintSource {
    fn default() -> Self {
        Self::new()
    }
}
