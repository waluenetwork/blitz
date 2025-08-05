
use anyhow::Result;
use gl::types::*;
use rustc_hash::FxHashMap;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct Surface {
    pub id: u64,
    pub texture_id: GLuint,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub visible: bool,
}

pub struct SurfaceManager {
    surfaces: FxHashMap<u64, Surface>,
    next_surface_id: u64,
}

impl SurfaceManager {
    pub fn new() -> Self {
        debug!("Creating surface manager");
        
        Self {
            surfaces: FxHashMap::default(),
            next_surface_id: 1,
        }
    }
    
    pub fn create_surface(&mut self, width: u32, height: u32) -> Result<u64> {
        let surface_id = self.next_surface_id;
        self.next_surface_id += 1;
        
        let texture_id = self.create_texture(width, height)?;
        
        let surface = Surface {
            id: surface_id,
            texture_id,
            width,
            height,
            x: 0,
            y: 0,
            visible: true,
        };
        
        self.surfaces.insert(surface_id, surface);
        
        debug!("Created surface {} ({}x{})", surface_id, width, height);
        Ok(surface_id)
    }
    
    pub fn destroy_surface(&mut self, surface_id: u64) -> Result<()> {
        if let Some(surface) = self.surfaces.remove(&surface_id) {
            unsafe {
                gl::DeleteTextures(1, &surface.texture_id);
            }
            debug!("Destroyed surface {}", surface_id);
        }
        Ok(())
    }
    
    pub fn update_surface_position(&mut self, surface_id: u64, x: i32, y: i32) -> Result<()> {
        if let Some(surface) = self.surfaces.get_mut(&surface_id) {
            surface.x = x;
            surface.y = y;
            debug!("Updated surface {} position to ({}, {})", surface_id, x, y);
        }
        Ok(())
    }
    
    pub fn set_surface_visibility(&mut self, surface_id: u64, visible: bool) -> Result<()> {
        if let Some(surface) = self.surfaces.get_mut(&surface_id) {
            surface.visible = visible;
            debug!("Set surface {} visibility to {}", surface_id, visible);
        }
        Ok(())
    }
    
    pub fn render_surfaces(&self) -> Result<()> {
        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        
        for surface in self.surfaces.values() {
            if surface.visible {
                self.render_surface(surface)?;
            }
        }
        
        Ok(())
    }
    
    fn render_surface(&self, surface: &Surface) -> Result<()> {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, surface.texture_id);
            
            debug!("Rendering surface {} at ({}, {})", surface.id, surface.x, surface.y);
            
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
        
        Ok(())
    }
    
    fn create_texture(&self, width: u32, height: u32) -> Result<GLuint> {
        unsafe {
            let mut texture_id = 0;
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
            
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );
            
            gl::BindTexture(gl::TEXTURE_2D, 0);
            
            debug!("Created texture {} ({}x{})", texture_id, width, height);
            Ok(texture_id)
        }
    }
    
    pub fn get_surface(&self, surface_id: u64) -> Option<&Surface> {
        self.surfaces.get(&surface_id)
    }
    
    pub fn list_surfaces(&self) -> Vec<u64> {
        self.surfaces.keys().copied().collect()
    }
}

impl Default for SurfaceManager {
    fn default() -> Self {
        Self::new()
    }
}
