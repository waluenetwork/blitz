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
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    fn render_primitive_to_cpu_buffer(
        image_data: &mut [u8],
        primitive: &egui::ClippedPrimitive,
        width: u32,
        height: u32,
    ) {
        let egui::ClippedPrimitive { clip_rect, primitive } = primitive;
        
        println!("DEBUG: Processing primitive with clip_rect: {:?}", clip_rect);
        
        match primitive {
            egui::epaint::Primitive::Mesh(mesh) => {
                println!("DEBUG: Found mesh with {} vertices, {} indices", 
                         mesh.vertices.len(), mesh.indices.len());
                
                if !mesh.vertices.is_empty() && !mesh.indices.is_empty() {
                    println!("DEBUG: Rendering {} triangles to CPU buffer", mesh.indices.len() / 3);
                    
                    for vertex in &mesh.vertices {
                        let x = vertex.pos.x as i32;
                        let y = vertex.pos.y as i32;
                        let color = vertex.color;
                        
                        for dy in -2..=2 {
                            for dx in -2..=2 {
                                let px = x + dx;
                                let py = y + dy;
                                
                                if px >= 0 && py >= 0 && px < width as i32 && py < height as i32 {
                                    let idx = ((py as u32 * width + px as u32) * 4) as usize;
                                    if idx + 3 < image_data.len() {
                                        image_data[idx] = color.r();
                                        image_data[idx + 1] = color.g();
                                        image_data[idx + 2] = color.b();
                                        image_data[idx + 3] = color.a();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            egui::epaint::Primitive::Callback(_) => {
                println!("DEBUG: Skipping callback primitive");
            }
        }
    }
}

impl CustomPaintSource for EguiPaintSource {
    fn resume(&mut self, _instance: &Instance, device_handle: &DeviceHandle) {
        println!("DEBUG: EguiPaintSource::resume called");
        
        self.state = EguiRendererState::Active {
            device: device_handle.device.clone(),
            queue: device_handle.queue.clone(),
            texture: None,
            texture_handle: None,
        };
        
        println!("DEBUG: EguiPaintSource::resume completed");
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

        self.process_input();

        let &mut EguiRendererState::Active {
            ref device,
            ref queue,
            ref mut texture,
            ref mut texture_handle,
        } = &mut self.state
        else {
            println!("DEBUG: EguiPaintSource::render - state not active");
            return None;
        };

        println!("DEBUG: EguiPaintSource::render - state is active, proceeding");

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

        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui_encoder"),
        });

        let mut image_data = vec![0u8; (width * height * 4) as usize];
        
        let bg_color = if shapes_count > 0 { [200, 100, 150, 255] } else { [50, 50, 50, 255] };
        for chunk in image_data.chunks_mut(4) {
            chunk.copy_from_slice(&bg_color);
        }
        
        for primitive in &clipped_primitives {
            Self::render_primitive_to_cpu_buffer(&mut image_data, primitive, width, height);
        }
        
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: texture_ref,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(Some(encoder.finish()));
        println!("DEBUG: EguiPaintSource::render completed successfully");

        Some(handle)
    }
}

impl Default for EguiPaintSource {
    fn default() -> Self {
        Self::new()
    }
}
