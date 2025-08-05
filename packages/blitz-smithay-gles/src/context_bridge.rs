
use anyhow::Result;
use anyrender_gles::GlContext;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tracing::{debug, error};

pub struct ContextBridge {
    gl_context: GlContext,
    is_current: bool,
}

impl ContextBridge {
    pub fn new<W>(window: &W) -> Result<Self>
    where
        W: HasWindowHandle + HasDisplayHandle + ?Sized,
    {
        debug!("Creating OpenGL context bridge");
        
        let gl_context = GlContext::new(window)?;
        
        Ok(Self {
            gl_context,
            is_current: false,
        })
    }
    
    pub fn make_current(&mut self) -> Result<()> {
        if !self.is_current {
            self.gl_context.make_current()?;
            self.is_current = true;
            debug!("OpenGL context made current");
        }
        Ok(())
    }
    
    pub fn swap_buffers(&self) -> Result<()> {
        self.gl_context.swap_buffers()?;
        debug!("Swapped OpenGL buffers");
        Ok(())
    }
    
    pub fn resize(&self, width: u32, height: u32) -> Result<()> {
        self.gl_context.resize(width, height)?;
        debug!("Resized OpenGL context to {}x{}", width, height);
        Ok(())
    }
    
    pub fn get_framebuffer_size(&self) -> (u32, u32) {
        self.gl_context.get_framebuffer_size()
    }
}
