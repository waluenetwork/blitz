
use anyhow::Result;
use gl::types::*;
use rustc_hash::FxHashMap;
use tracing::debug;
use std::ffi::CString;

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
    shader_program: Option<GLuint>,
    vao: GLuint,
    vbo: GLuint,
    ebo: GLuint,
}

impl SurfaceManager {
    pub fn new() -> Self {
        debug!("Creating surface manager");
        
        let mut manager = Self {
            surfaces: FxHashMap::default(),
            next_surface_id: 1,
            shader_program: None,
            vao: 0,
            vbo: 0,
            ebo: 0,
        };
        
        manager.init_rendering_resources();
        manager
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
        if let Some(program) = self.shader_program {
            unsafe {
                gl::UseProgram(program);
                gl::BindTexture(gl::TEXTURE_2D, surface.texture_id);
                gl::BindVertexArray(self.vao);
                
                debug!("Rendering surface {} at ({}, {})", surface.id, surface.x, surface.y);
                
                gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
                
                gl::BindVertexArray(0);
                gl::BindTexture(gl::TEXTURE_2D, 0);
                gl::UseProgram(0);
                
                debug!("Successfully rendered surface {} texture to screen", surface.id);
            }
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
    
    fn init_rendering_resources(&mut self) {
        unsafe {
            let vertex_shader_source = CString::new(r#"
                #version 300 es
                precision mediump float;
                
                layout (location = 0) in vec2 aPos;
                layout (location = 1) in vec2 aTexCoord;
                
                out vec2 TexCoord;
                
                void main() {
                    gl_Position = vec4(aPos, 0.0, 1.0);
                    TexCoord = aTexCoord;
                }
            "#).unwrap();
            
            let fragment_shader_source = CString::new(r#"
                #version 300 es
                precision mediump float;
                
                in vec2 TexCoord;
                out vec4 FragColor;
                
                uniform sampler2D ourTexture;
                
                void main() {
                    FragColor = texture(ourTexture, TexCoord);
                }
            "#).unwrap();
            
            let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
            gl::ShaderSource(vertex_shader, 1, &vertex_shader_source.as_ptr(), std::ptr::null());
            gl::CompileShader(vertex_shader);
            
            let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            gl::ShaderSource(fragment_shader, 1, &fragment_shader_source.as_ptr(), std::ptr::null());
            gl::CompileShader(fragment_shader);
            
            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);
            
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
            
            self.shader_program = Some(program);
            
            let vertices: [f32; 16] = [
                -1.0, -1.0,    0.0, 1.0,
                 1.0, -1.0,    1.0, 1.0,
                 1.0,  1.0,    1.0, 0.0,
                -1.0,  1.0,    0.0, 0.0,
            ];
            
            let indices: [u32; 6] = [
                0, 1, 2,
                2, 3, 0
            ];
            
            gl::GenVertexArrays(1, &mut self.vao);
            gl::GenBuffers(1, &mut self.vbo);
            gl::GenBuffers(1, &mut self.ebo);
            
            gl::BindVertexArray(self.vao);
            
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * std::mem::size_of::<u32>()) as isize,
                indices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 4 * std::mem::size_of::<f32>() as i32, std::ptr::null());
            gl::EnableVertexAttribArray(0);
            
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 4 * std::mem::size_of::<f32>() as i32, (2 * std::mem::size_of::<f32>()) as *const _);
            gl::EnableVertexAttribArray(1);
            
            gl::BindVertexArray(0);
            
            debug!("Initialized surface rendering resources with shader program {}", program);
        }
    }
}

impl Default for SurfaceManager {
    fn default() -> Self {
        Self::new()
    }
}
