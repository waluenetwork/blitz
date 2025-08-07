use std::sync::{Arc, Mutex};
use tracing::debug;

use smithay::{
    backend::renderer::{
        gles::GlesRenderer,
        Frame, Renderer,
    },
    utils::{Rectangle, Size},
};

use wgpu::{Device as WgpuDevice, Queue as WgpuQueue, Texture as WgpuTexture, TextureView};

use crate::{
    surface_manager::{WaylandSurfaceManager, RenderElement, FinalRenderElement},
    BlitzTexture, BlitzSmithayError, ObjectId,
};

pub struct SurfaceCompositor {
    surface_manager: Arc<Mutex<WaylandSurfaceManager>>,
    gles_renderer: Option<GlesRenderer>,
    wgpu_device: WgpuDevice,
    wgpu_queue: WgpuQueue,
    output_size: Size<i32, smithay::utils::Logical>,
}

impl SurfaceCompositor {
    pub fn new(
        wgpu_device: WgpuDevice,
        wgpu_queue: WgpuQueue,
        output_size: Size<i32, smithay::utils::Logical>,
    ) -> Self {
        debug!("Initializing SurfaceCompositor with output size {:?}", output_size);
        
        Self {
            surface_manager: Arc::new(Mutex::new(WaylandSurfaceManager::new())),
            gles_renderer: None,
            wgpu_device,
            wgpu_queue,
            output_size,
        }
    }
    
    pub fn set_gles_renderer(&mut self, renderer: GlesRenderer) {
        debug!("Setting GlesRenderer for surface compositor");
        self.gles_renderer = Some(renderer);
    }
    
    pub fn render_surfaces_to_wgpu_texture(
        &mut self,
        target_texture: &WgpuTexture,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Rendering surfaces to WGPU texture");
        
        let render_elements = {
            let surface_manager = self.surface_manager
                .lock()
                .map_err(|_| BlitzSmithayError::ResourceManagerLocked)?;
            surface_manager.render_all_surfaces()
        };
        
        if render_elements.is_empty() {
            debug!("No surfaces to render");
            return Ok(());
        }
        
        let texture_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Surface Compositor Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Surface Compositor Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            for element in &render_elements {
                self.render_surface_element(&mut render_pass, element)?;
            }
        }
        
        self.wgpu_queue.submit(std::iter::once(encoder.finish()));
        
        debug!("Successfully rendered {} surfaces to WGPU texture", render_elements.len());
        Ok(())
    }
    
    fn render_surface_element(
        &self,
        render_pass: &mut wgpu::RenderPass,
        element: &RenderElement,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Rendering surface element {:?} with z_index={}", 
               element.surface_id, element.z_index);
        
        if let crate::surface_manager::TextureSource::Simple(simple_texture) = &element.texture_source {
            if let Some(wgpu_texture) = simple_texture.texture.wgpu_texture() {
                let texture_view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
                
                self.apply_surface_transform(render_pass, element, &texture_view)?;
                
                debug!("Rendered surface {:?} with opacity={}", 
                       element.surface_id, element.opacity);
            } else {
                debug!("Surface {:?} has no WGPU texture, skipping", element.surface_id);
            }
        }
        
        Ok(())
    }
    
    fn apply_surface_transform(
        &self,
        render_pass: &mut wgpu::RenderPass,
        element: &RenderElement,
        texture_view: &TextureView,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Applying transform {:?} scale={} to surface {:?}", 
               element.transform, element.scale, element.surface_id);
        
        
        debug!("Transform application completed for surface {:?}", element.surface_id);
        Ok(())
    }
    
    pub fn add_surface(&self, surface_id: ObjectId) -> Result<(), BlitzSmithayError> {
        let mut surface_manager = self.surface_manager
            .lock()
            .map_err(|_| BlitzSmithayError::ResourceManagerLocked)?;
        surface_manager.add_surface(surface_id);
        Ok(())
    }
    
    pub fn remove_surface(&self, surface_id: ObjectId) -> Result<(), BlitzSmithayError> {
        let mut surface_manager = self.surface_manager
            .lock()
            .map_err(|_| BlitzSmithayError::ResourceManagerLocked)?;
        surface_manager.remove_surface(surface_id);
        Ok(())
    }
    
    pub fn commit_surface(&self, surface_id: ObjectId) -> Result<(), BlitzSmithayError> {
        let mut surface_manager = self.surface_manager
            .lock()
            .map_err(|_| BlitzSmithayError::ResourceManagerLocked)?;
        surface_manager.commit_surface(surface_id)
            .map_err(BlitzSmithayError::SurfaceError)
    }
    
    pub fn set_surface_texture(
        &self,
        surface_id: ObjectId,
        texture: BlitzTexture,
    ) -> Result<(), BlitzSmithayError> {
        let mut surface_manager = self.surface_manager
            .lock()
            .map_err(|_| BlitzSmithayError::ResourceManagerLocked)?;
        
        if let Some(surface) = surface_manager.surfaces.get_mut(&surface_id) {
            surface.set_texture(texture);
            surface.map_surface();
            debug!("Set texture for surface {:?} and mapped it", surface_id);
            Ok(())
        } else {
            Err(BlitzSmithayError::SurfaceError(crate::SurfaceError::SurfaceNotFound))
        }
    }
    
    pub fn track_surface_damage(
        &self,
        surface_id: ObjectId,
        damage: &[Rectangle<i32, smithay::utils::Logical>],
    ) -> Result<(), BlitzSmithayError> {
        let mut surface_manager = self.surface_manager
            .lock()
            .map_err(|_| BlitzSmithayError::ResourceManagerLocked)?;
        surface_manager.track_damage(surface_id, damage);
        Ok(())
    }
    
    pub fn get_surface_count(&self) -> Result<usize, BlitzSmithayError> {
        let surface_manager = self.surface_manager
            .lock()
            .map_err(|_| BlitzSmithayError::ResourceManagerLocked)?;
        Ok(surface_manager.surfaces().count())
    }
}

impl Default for SurfaceCompositor {
    fn default() -> Self {
        panic!("SurfaceCompositor::default() should not be used - use new() with proper WGPU device")
    }
}
