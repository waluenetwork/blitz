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
                    gl::Disable(gl::DEPTH_TEST);
                    gl::Disable(gl::CULL_FACE);
                    gl::ClearColor(0.2, 0.2, 0.2, 1.0);
                    gl::Viewport(0, 0, width as i32, height as i32);
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
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            
            let mut viewport = [0i32; 4];
            gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
            println!("🔍 Debug - Viewport set to: {}x{} at ({}, {})", viewport[2], viewport[3], viewport[0], viewport[1]);
            
            let fb_status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            println!("🔍 Debug - Framebuffer status: 0x{:x} (complete=0x{:x})", fb_status, gl::FRAMEBUFFER_COMPLETE);
            
            let mut current_fb = 0;
            gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut current_fb);
            println!("🔍 Debug - Current framebuffer: {}", current_fb);
            
            let mut clear_color = [0.0f32; 4];
            gl::GetFloatv(gl::COLOR_CLEAR_VALUE, clear_color.as_mut_ptr());
            println!("🔍 Debug - Clear color: {:?}", clear_color);
            
            gl::Clear(gl::COLOR_BUFFER_BIT);
            
            let mut depth_test = 0;
            let mut cull_face = 0;
            let mut blend = 0;
            let mut scissor_test = 0;
            gl::GetIntegerv(gl::DEPTH_TEST, &mut depth_test);
            gl::GetIntegerv(gl::CULL_FACE, &mut cull_face);
            gl::GetIntegerv(gl::BLEND, &mut blend);
            gl::GetIntegerv(gl::SCISSOR_TEST, &mut scissor_test);
            println!("🔍 Debug - GL State before render: depth={}, cull={}, blend={}, scissor={}", 
                     depth_test, cull_face, blend, scissor_test);
            
            if blend != 0 {
                let mut blend_src = 0;
                let mut blend_dst = 0;
                gl::GetIntegerv(gl::BLEND_SRC_ALPHA, &mut blend_src);
                gl::GetIntegerv(gl::BLEND_DST_ALPHA, &mut blend_dst);
                println!("🔍 Debug - Blend func: src=0x{:x}, dst=0x{:x}", blend_src, blend_dst);
            }
            
            let error = gl::GetError();
            if error != gl::NO_ERROR {
                println!("⚠️  OpenGL error before rendering: 0x{:x}", error);
            } else {
                println!("✅ Pre-render state validation passed");
            }
            
            println!("🧹 Set viewport to {}x{} and cleared GL buffers", self.width, self.height);
        }

        println!("🎨 Calling draw function...");
        draw_fn(&mut scene_painter);
        println!("🎨 Draw function completed");

        unsafe {
            gl::Finish();
            
            gl::Flush();
        }
        
        if let Err(e) = context.swap_buffers() {
            tracing::error!("Failed to swap buffers: {}", e);
        } else {
            println!("🔄 Swapped GL buffers successfully - content should now be visible");
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
