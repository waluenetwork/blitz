use crate::{gl_context::GlContext, scene::GlesScenePainter};
use anyrender::{WindowHandle, WindowRenderer};
use std::sync::Arc;

pub struct GlesWindowRenderer {
    gl_context: Option<GlContext>,
    window_handle: Option<Arc<dyn WindowHandle>>,
    width: u32,
    height: u32,
    is_active: bool,
}

impl GlesWindowRenderer {
    pub fn new() -> Self {
        Self {
            gl_context: None,
            window_handle: None,
            width: 800,
            height: 600,
            is_active: false,
        }
    }
}

impl WindowRenderer for GlesWindowRenderer {
    type ScenePainter<'a> = GlesScenePainter where Self: 'a;

    fn resume(&mut self, window_handle: Arc<dyn WindowHandle>, width: u32, height: u32) {
        self.window_handle = Some(window_handle.clone());
        self.width = width;
        self.height = height;

        match GlContext::new(window_handle.as_ref()) {
            Ok(context) => {
                self.gl_context = Some(context);
                self.is_active = true;
                
                unsafe {
                    gl::Enable(gl::BLEND);
                    gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                    gl::ClearColor(1.0, 1.0, 1.0, 1.0);
                }
                
                tracing::info!("GLES window renderer resumed with size {}x{}", width, height);
            }
            Err(e) => {
                tracing::error!("Failed to create GL context: {}", e);
                self.is_active = false;
            }
        }
    }

    fn suspend(&mut self) {
        self.gl_context = None;
        self.is_active = false;
        tracing::info!("GLES window renderer suspended");
    }

    fn is_active(&self) -> bool {
        self.is_active
    }

    fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        
        if let Some(ref context) = self.gl_context {
            if let Err(e) = context.resize(width, height) {
                tracing::error!("Failed to resize GL context: {}", e);
            }
        }
    }

    fn render<F: FnOnce(&mut Self::ScenePainter<'_>)>(&mut self, draw_fn: F) {
        println!("🖼️  GlesWindowRenderer::render() called with size {}x{}", self.width, self.height);
        
        let Some(ref context) = self.gl_context else {
            println!("❌ No GL context available");
            return;
        };

        if let Err(e) = context.make_current() {
            tracing::error!("Failed to make GL context current: {}", e);
            return;
        }

        let mut scene_painter = match GlesScenePainter::new(self.width, self.height) {
            Ok(painter) => {
                println!("✅ Created GlesScenePainter successfully");
                painter
            },
            Err(e) => {
                tracing::error!("Failed to create scene painter: {}", e);
                return;
            }
        };

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            println!("🧹 Cleared GL buffers");
        }

        println!("🎨 Calling draw function...");
        draw_fn(&mut scene_painter);
        println!("🎨 Draw function completed");

        if let Err(e) = context.swap_buffers() {
            tracing::error!("Failed to swap buffers: {}", e);
        } else {
            println!("🔄 Swapped GL buffers successfully");
        }
    }

    fn forward_event_to_custom_paint_source(&mut self, _id: u64, _x: f32, _y: f32, _event_type: &str) -> bool {
        false
    }
    
    fn forward_key_event_to_custom_paint_source(&mut self, _id: u64, _key_event: &dyn std::any::Any) -> bool {
        false
    }
    
    fn forward_ime_event_to_custom_paint_source(&mut self, _id: u64, _ime_event: &dyn std::any::Any) -> bool {
        false
    }
}

impl GlesWindowRenderer {
    pub fn get_window_handle(&self) -> Option<Arc<dyn WindowHandle>> {
        self.window_handle.clone()
    }
}

impl Default for GlesWindowRenderer {
    fn default() -> Self {
        Self::new()
    }
}
