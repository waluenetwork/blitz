use anyrender_vello::wgpu_context::DeviceHandle;
use anyrender_vello::{CustomPaintCtx, CustomPaintSource, TextureHandle};
use blitz_smithay::BlitzSmithayRenderer;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Instant;
use wgpu::Instance;
use tracing::debug;

pub struct SmithayPaintSource {
    state: SmithayRendererState,
    start_time: Instant,
    tx: Sender<SmithayMessage>,
    rx: Receiver<SmithayMessage>,
    blitz_renderer: Option<BlitzSmithayRenderer>,
}

pub enum SmithayMessage {
    SurfaceCreated(u32, u32),
    SurfaceDestroyed,
    UpdateContent,
}

enum SmithayRendererState {
    Active(Box<ActiveSmithayRenderer>),
    Suspended,
}

struct ActiveSmithayRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    displayed_texture: Option<TextureAndHandle>,
    next_texture: Option<TextureAndHandle>,
    compositor_texture: Option<wgpu::Texture>,
}

#[derive(Clone)]
struct TextureAndHandle {
    texture: wgpu::Texture,
    handle: TextureHandle,
}

impl CustomPaintSource for SmithayPaintSource {
    fn resume(&mut self, _instance: &Instance, device_handle: &DeviceHandle) {
        debug!("DEBUG: SmithayPaintSource resuming with WGPU device");
        let active_state = ActiveSmithayRenderer::new(device_handle);
        self.state = SmithayRendererState::Active(Box::new(active_state));
        
        if let Ok(renderer) = BlitzSmithayRenderer::new() {
            self.blitz_renderer = Some(renderer);
            debug!("DEBUG: BlitzSmithayRenderer initialized in paint source");
        }
    }

    fn suspend(&mut self) {
        debug!("DEBUG: SmithayPaintSource suspending");
        self.state = SmithayRendererState::Suspended;
    }

    fn render(&mut self, ctx: CustomPaintCtx<'_>, width: u32, height: u32, _scale: f64) -> Option<TextureHandle> {
        self.process_messages();
        self.render_compositor_content(ctx, width, height)
    }
}

impl SmithayPaintSource {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        debug!("DEBUG: Creating SmithayPaintSource");
        let (tx, rx) = channel();
        
        Ok(Self {
            state: SmithayRendererState::Suspended,
            start_time: Instant::now(),
            tx,
            rx,
            blitz_renderer: None,
        })
    }
    
    pub fn sender(&self) -> Sender<SmithayMessage> {
        self.tx.clone()
    }
    
    fn process_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                SmithayMessage::SurfaceCreated(width, height) => {
                    debug!("DEBUG: Processing surface creation {}x{}", width, height);
                }
                SmithayMessage::SurfaceDestroyed => {
                    debug!("DEBUG: Processing surface destruction");
                }
                SmithayMessage::UpdateContent => {
                    debug!("DEBUG: Processing content update");
                }
            }
        }
    }
    
    fn render_compositor_content(&mut self, mut ctx: CustomPaintCtx<'_>, width: u32, height: u32) -> Option<TextureHandle> {
        if width == 0 || height == 0 {
            return None;
        }
        
        let SmithayRendererState::Active(state) = &mut self.state else {
            return None;
        };
        
        let needs_new_texture = match &state.next_texture {
            Some(next) => next.texture.width() != width || next.texture.height() != height,
            None => true,
        };
        
        if needs_new_texture {
            if let Some(old) = &state.next_texture {
                ctx.unregister_texture(old.handle);
            }
            
            let texture = create_compositor_texture(&state.device, width, height);
            let handle = ctx.register_texture(texture.clone());
            state.next_texture = Some(TextureAndHandle { texture, handle });
        }
        
        let texture_handle = state.next_texture.as_ref().unwrap().handle;
        let target_texture = state.next_texture.as_ref().unwrap().texture.clone();
        
        let device = state.device.clone();
        let queue = state.queue.clone();
        
        Self::render_to_texture(&device, &queue, &target_texture);
        
        std::mem::swap(&mut state.next_texture, &mut state.displayed_texture);
        Some(texture_handle)
    }
    
    fn render_to_texture(device: &wgpu::Device, queue: &wgpu::Queue, target_texture: &wgpu::Texture) {
        debug!("DEBUG: Rendering Smithay compositor content to WGPU texture");
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Smithay Compositor Render"),
        });
        
        {
            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Smithay Compositor Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_texture.create_view(&wgpu::TextureViewDescriptor::default()),
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
            
            drop(rpass);
        }
        
        queue.submit(Some(encoder.finish()));
    }
}

impl ActiveSmithayRenderer {
    pub fn new(device_handle: &DeviceHandle) -> Self {
        debug!("DEBUG: Creating ActiveSmithayRenderer");
        
        Self {
            device: device_handle.device.clone(),
            queue: device_handle.queue.clone(),
            displayed_texture: None,
            next_texture: None,
            compositor_texture: None,
        }
    }
}

fn create_compositor_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    debug!("DEBUG: Creating compositor texture {}x{}", width, height);
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Smithay Compositor Texture"),
        size: wgpu::Extent3d { 
            width, 
            height, 
            depth_or_array_layers: 1 
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
