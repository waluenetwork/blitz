use anyrender_vello::wgpu_context::DeviceHandle;
use anyrender_vello::{CustomPaintCtx, CustomPaintSource, TextureHandle};
use egui::{Context, RawInput};
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
        while let Ok(_input) = self.rx.try_recv() {
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
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }
}

impl CustomPaintSource for EguiPaintSource {
    fn resume(&mut self, _instance: &Instance, device_handle: &DeviceHandle) {
        self.state = EguiRendererState::Active {
            device: device_handle.device.clone(),
            queue: device_handle.queue.clone(),
            texture: None,
            texture_handle: None,
        };
    }

    fn suspend(&mut self) {
        self.state = EguiRendererState::Suspended;
    }

    fn render(
        &mut self,
        mut ctx: CustomPaintCtx<'_>,
        width: u32,
        height: u32,
        _scale: f64,
    ) -> Option<TextureHandle> {
        if width == 0 || height == 0 {
            return None;
        }

        self.process_input();

        let &mut EguiRendererState::Active {
            ref device,
            ref queue,
            ref mut texture,
            ref mut texture_handle,
        } = &mut self.state
        else {
            return None;
        };

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
        }

        let texture_ref = texture.as_ref().unwrap();
        let handle = texture_handle.unwrap();

        let _full_output = self.egui_ctx.run(egui::RawInput::default(), |ctx| {
            if let Some(ref ui_fn) = self.ui_fn {
                ui_fn(ctx);
            } else {
                egui::Window::new("Egui Demo").show(ctx, |ui| {
                    ui.label("Hello from egui in Blitz!");
                    let _ = ui.button("Click me");
                });
            }
        });

        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("egui_encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_ref.create_view(&wgpu::TextureViewDescriptor::default()),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
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
