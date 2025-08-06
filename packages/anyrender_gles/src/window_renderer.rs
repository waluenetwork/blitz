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
        let Some(ref context) = self.gl_context else {
            return;
        };

        if let Err(e) = context.make_current() {
            tracing::error!("Failed to make GL context current: {}", e);
            return;
        }

        let mut framebuffer = 0;
        let mut color_texture = 0;
        let mut presentation_vao = 0;
        let mut presentation_vbo = 0;
        
        unsafe {
            gl::GenTextures(1, &mut color_texture);
            gl::BindTexture(gl::TEXTURE_2D, color_texture);
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, gl::RGBA as i32,
                self.width as i32, self.height as i32, 0,
                gl::RGBA, gl::UNSIGNED_BYTE, std::ptr::null()
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D, color_texture, 0
            );
            
            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                tracing::error!("Intermediate framebuffer not complete: 0x{:x}", status);
                gl::DeleteFramebuffers(1, &framebuffer);
                gl::DeleteTextures(1, &color_texture);
                return;
            }
            
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        let mut scene_painter = match GlesScenePainter::new(self.width, self.height) {
            Ok(painter) => painter,
            Err(e) => {
                tracing::error!("Failed to create scene painter: {}", e);
                unsafe {
                    gl::DeleteFramebuffers(1, &framebuffer);
                    gl::DeleteTextures(1, &color_texture);
                }
                return;
            }
        };

        draw_fn(&mut scene_painter);

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            
            let quad_vertices: [f32; 16] = [
                -1.0, -1.0,  0.0, 1.0,  // bottom-left
                 1.0, -1.0,  1.0, 1.0,  // bottom-right
                 1.0,  1.0,  1.0, 0.0,  // top-right
                -1.0,  1.0,  0.0, 0.0,  // top-left
            ];
            
            gl::GenVertexArrays(1, &mut presentation_vao);
            gl::GenBuffers(1, &mut presentation_vbo);
            
            gl::BindVertexArray(presentation_vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, presentation_vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (quad_vertices.len() * std::mem::size_of::<f32>()) as isize,
                quad_vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 4 * std::mem::size_of::<f32>() as i32, std::ptr::null());
            gl::EnableVertexAttribArray(0);
            
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 4 * std::mem::size_of::<f32>() as i32, (2 * std::mem::size_of::<f32>()) as *const _);
            gl::EnableVertexAttribArray(1);
            
            if let Ok(program) = scene_painter.use_texture_shader() {
                gl::UseProgram(program);
                gl::BindTexture(gl::TEXTURE_2D, color_texture);
                gl::Uniform1i(gl::GetUniformLocation(program, b"u_texture\0".as_ptr() as *const i8), 0);
                
                gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
            }
            
            gl::Finish();
            
            gl::DeleteVertexArrays(1, &presentation_vao);
            gl::DeleteBuffers(1, &presentation_vbo);
            gl::DeleteFramebuffers(1, &framebuffer);
            gl::DeleteTextures(1, &color_texture);
        }
        
        if let Err(e) = context.swap_buffers() {
            tracing::error!("Failed to swap buffers: {}", e);
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
