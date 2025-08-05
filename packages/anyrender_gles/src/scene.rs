use crate::{
    shader_manager::{ShaderManager, ShaderType},
    tessellation::{GlesTessellator, Vertex},
    text_renderer::GlesTextRenderer,
};
use anyrender::{NormalizedCoord, Paint, PaintScene};
use gl::types::*;
use kurbo::{Affine, BezPath, Rect, Shape, Stroke};
use peniko::{BlendMode, BrushRef, Color, Fill, Font, StyleRef};
use rustc_hash::FxHashMap;
use std::mem;

pub struct GlesScenePainter {
    shader_manager: ShaderManager,
    tessellator: GlesTessellator,
    text_renderer: GlesTextRenderer,
    
    vao: GLuint,
    vbo: GLuint,
    ebo: GLuint,
    
    transform_stack: Vec<Affine>,
    current_transform: Affine,
    
    projection_matrix: [f32; 16],
    
    layer_stack: Vec<LayerState>,
    
    custom_paint_sources: FxHashMap<u64, ()>,
}

#[derive(Debug, Clone)]
struct LayerState {
    blend_mode: BlendMode,
    alpha: f32,
    transform: Affine,
    clip_rect: Option<Rect>,
}

impl GlesScenePainter {
    pub fn new(width: u32, height: u32) -> anyhow::Result<Self> {
        let shader_manager = ShaderManager::new()?;
        let tessellator = GlesTessellator::new();
        let text_renderer = GlesTextRenderer::new()?;
        
        let mut vao = 0;
        let mut vbo = 0;
        let mut ebo = 0;
        
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);
            
            gl::BindVertexArray(vao);
            
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                0 as *const _,
            );
            gl::EnableVertexAttribArray(0);
            
            gl::VertexAttribPointer(
                1,
                4,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                (2 * mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);
            
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BindVertexArray(0);
        }
        
        let projection_matrix = Self::create_orthographic_matrix(width as f32, height as f32);
        
        Ok(Self {
            shader_manager,
            tessellator,
            text_renderer,
            vao,
            vbo,
            ebo,
            transform_stack: Vec::new(),
            current_transform: Affine::IDENTITY,
            projection_matrix,
            layer_stack: Vec::new(),
            custom_paint_sources: FxHashMap::default(),
        })
    }
    
    fn create_orthographic_matrix(width: f32, height: f32) -> [f32; 16] {
        [
            2.0 / width, 0.0, 0.0, 0.0,
            0.0, -2.0 / height, 0.0, 0.0,
            0.0, 0.0, -1.0, 0.0,
            -1.0, 1.0, 0.0, 1.0,
        ]
    }
    
    fn set_transform_uniforms(&self, shader_type: ShaderType) -> anyhow::Result<()> {
        let _program = self.shader_manager.use_program(shader_type)?;
        
        let transform_matrix = self.affine_to_matrix4(self.current_transform);
        
        unsafe {
            let transform_loc = self.shader_manager.get_uniform_location(shader_type, "u_transform")?;
            let projection_loc = self.shader_manager.get_uniform_location(shader_type, "u_projection")?;
            
            gl::UniformMatrix4fv(transform_loc, 1, gl::FALSE, transform_matrix.as_ptr());
            gl::UniformMatrix4fv(projection_loc, 1, gl::FALSE, self.projection_matrix.as_ptr());
        }
        
        Ok(())
    }
    
    fn affine_to_matrix4(&self, affine: Affine) -> [f32; 16] {
        let coeffs = affine.as_coeffs();
        [
            coeffs[0] as f32, coeffs[1] as f32, 0.0, 0.0,
            coeffs[2] as f32, coeffs[3] as f32, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            coeffs[4] as f32, coeffs[5] as f32, 0.0, 1.0,
        ]
    }
    
    fn upload_geometry(&self, vertices: &[Vertex], indices: &[u32]) {
        unsafe {
            gl::BindVertexArray(self.vao);
            
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<Vertex>()) as isize,
                vertices.as_ptr() as *const _,
                gl::DYNAMIC_DRAW,
            );
            
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * mem::size_of::<u32>()) as isize,
                indices.as_ptr() as *const _,
                gl::DYNAMIC_DRAW,
            );
        }
    }
    
    fn draw_elements(&self, count: usize) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawElements(gl::TRIANGLES, count as i32, gl::UNSIGNED_INT, std::ptr::null());
            
            let error = gl::GetError();
            if error != gl::NO_ERROR {
                println!("❌ OpenGL error after DrawElements: 0x{:x}", error);
            } else {
                println!("✅ DrawElements completed successfully for {} indices", count);
            }
        }
    }
    
    fn render_text_quads(&self, quads: &[crate::text_renderer::TextQuad]) {
        use crate::text_renderer::TextQuad;
        
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        for (i, quad) in quads.iter().enumerate() {
            let base_index = (i * 4) as u32;
            
            let quad_vertices = [
                quad.position[0], quad.position[1] + quad.size[1], // position
                0.0, 1.0, // texcoord
                quad.color[0], quad.color[1], quad.color[2], quad.color[3], // color
                
                quad.position[0] + quad.size[0], quad.position[1] + quad.size[1], // position
                1.0, 1.0, // texcoord
                quad.color[0], quad.color[1], quad.color[2], quad.color[3], // color
                
                quad.position[0] + quad.size[0], quad.position[1], // position
                1.0, 0.0, // texcoord
                quad.color[0], quad.color[1], quad.color[2], quad.color[3], // color
                
                quad.position[0], quad.position[1], // position
                0.0, 0.0, // texcoord
                quad.color[0], quad.color[1], quad.color[2], quad.color[3], // color
            ];
            
            vertices.extend_from_slice(&quad_vertices);
            
            let quad_indices = [
                base_index, base_index + 1, base_index + 2,
                base_index, base_index + 2, base_index + 3,
            ];
            indices.extend_from_slice(&quad_indices);
        }
        
        if vertices.is_empty() || indices.is_empty() {
            println!("⚪ No text vertices/indices to render");
            return;
        }
        
        println!("🔤 Rendering text with {} vertices, {} indices", vertices.len() / 8, indices.len());
        
        unsafe {
            gl::BindVertexArray(self.vao);
            
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const _,
                gl::DYNAMIC_DRAW,
            );
            
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 8 * mem::size_of::<f32>() as i32, 0 as *const _);
            gl::EnableVertexAttribArray(0);
            
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, 8 * mem::size_of::<f32>() as i32, (2 * mem::size_of::<f32>()) as *const _);
            gl::EnableVertexAttribArray(1);
            
            gl::VertexAttribPointer(2, 4, gl::FLOAT, gl::FALSE, 8 * mem::size_of::<f32>() as i32, (4 * mem::size_of::<f32>()) as *const _);
            gl::EnableVertexAttribArray(2);
            
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * mem::size_of::<u32>()) as isize,
                indices.as_ptr() as *const _,
                gl::DYNAMIC_DRAW,
            );
            
            if let Some(first_quad) = quads.first() {
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, first_quad.texture_id);
                
                if let Ok(texture_loc) = self.shader_manager.get_uniform_location(ShaderType::Text, "u_texture") {
                    gl::Uniform1i(texture_loc, 0);
                }
            }
            
            gl::DrawElements(gl::TRIANGLES, indices.len() as i32, gl::UNSIGNED_INT, std::ptr::null());
            
            let error = gl::GetError();
            if error != gl::NO_ERROR {
                println!("❌ OpenGL error after text DrawElements: 0x{:x}", error);
            } else {
                println!("✅ Text DrawElements completed successfully for {} indices", indices.len());
            }
        }
    }
    
    fn color_to_array(color: Color) -> [f32; 4] {
        let components = color.components;
        [
            components[0],
            components[1], 
            components[2],
            components[3],
        ]
    }
    
    fn brush_to_color(brush: BrushRef) -> [f32; 4] {
        match brush {
            BrushRef::Solid(color) => Self::color_to_array(color),
            BrushRef::Gradient(_) => [1.0, 0.0, 1.0, 1.0],
            BrushRef::Image(_) => [0.0, 1.0, 1.0, 1.0],
        }
    }
}

impl PaintScene for GlesScenePainter {
    fn reset(&mut self) {
        println!("🔄 GlesScenePainter::reset() called");
        self.transform_stack.clear();
        self.current_transform = Affine::IDENTITY;
        self.layer_stack.clear();
        
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }
    
    fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        clip: &impl Shape,
    ) {
        let layer = LayerState {
            blend_mode: blend.into(),
            alpha,
            transform,
            clip_rect: Some(clip.bounding_box()),
        };
        
        self.layer_stack.push(layer);
        
        self.current_transform = self.current_transform * transform;
    }
    
    fn pop_layer(&mut self) {
        if let Some(_layer) = self.layer_stack.pop() {
            if let Some(parent_layer) = self.layer_stack.last() {
                self.current_transform = parent_layer.transform;
            } else {
                self.current_transform = Affine::IDENTITY;
            }
        }
    }
    
    fn stroke<'a>(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: impl Into<BrushRef<'a>>,
        _brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let brush_ref = brush.into();
        let color = Self::brush_to_color(brush_ref);
        
        let mut path = BezPath::new();
        shape.path_elements(0.1).for_each(|el| path.push(el));
        
        let buffers = match self.tessellator.tessellate_stroke(&path, color, style.width as f32) {
            Ok(buffers) => buffers,
            Err(_) => return,
        };
        
        let old_transform = self.current_transform;
        self.current_transform = self.current_transform * transform;
        
        if self.set_transform_uniforms(ShaderType::Stroke).is_ok() {
            self.upload_geometry(&buffers.vertices, &buffers.indices);
            self.draw_elements(buffers.indices.len());
        }
        
        self.current_transform = old_transform;
    }
    
    fn fill<'a>(
        &mut self,
        _style: Fill,
        transform: Affine,
        paint: impl Into<Paint<'a>>,
        _brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let paint: Paint<'_> = paint.into();
        
        let color = match paint {
            Paint::Solid(color) => Self::color_to_array(color),
            Paint::Gradient(_) => [1.0, 0.0, 1.0, 1.0],
            Paint::Image(_) => [0.0, 1.0, 1.0, 1.0],
            Paint::Custom(_) => [1.0, 1.0, 0.0, 1.0],
        };
        
        println!("🎨 GlesScenePainter::fill() called with color: {:?}", color);
        
        let mut path = BezPath::new();
        shape.path_elements(0.1).for_each(|el| path.push(el));
        
        let buffers = match self.tessellator.tessellate_fill(&path, color) {
            Ok(buffers) => {
                println!("✅ Tessellation successful: {} vertices, {} indices", 
                    buffers.vertices.len(), buffers.indices.len());
                buffers
            },
            Err(e) => {
                println!("❌ Tessellation failed: {:?}", e);
                return;
            },
        };
        
        let old_transform = self.current_transform;
        self.current_transform = self.current_transform * transform;
        
        if self.set_transform_uniforms(ShaderType::Fill).is_ok() {
            println!("✅ Set transform uniforms successfully");
            self.upload_geometry(&buffers.vertices, &buffers.indices);
            self.draw_elements(buffers.indices.len());
            println!("✅ Drew {} elements", buffers.indices.len());
        } else {
            println!("❌ Failed to set transform uniforms");
        }
        
        self.current_transform = old_transform;
    }
    
    fn draw_glyphs<'a, 's: 'a>(
        &'s mut self,
        _font: &'a Font,
        font_size: f32,
        _hint: bool,
        _normalized_coords: &'a [NormalizedCoord],
        _style: impl Into<StyleRef<'a>>,
        brush: impl Into<BrushRef<'a>>,
        _brush_alpha: f32,
        transform: Affine,
        _glyph_transform: Option<Affine>,
        glyphs: impl Iterator<Item = anyrender::Glyph>,
    ) {
        let brush_ref = brush.into();
        let color = Self::brush_to_color(brush_ref);
        
        println!("🔤 GlesScenePainter::draw_glyphs() called with font_size: {}, color: {:?}", font_size, color);
        
        let glyph_vec: Vec<_> = glyphs.collect();
        if glyph_vec.is_empty() {
            println!("⚪ No glyphs to render, skipping");
            return;
        }
        
        let text = "Sample Text";
        let position = kurbo::Point::new(0.0, 0.0);
        
        let old_transform = self.current_transform;
        self.current_transform = self.current_transform * transform;
        
        match self.text_renderer.render_text(text, font_size, position, color) {
            Ok(quads) => {
                if quads.is_empty() {
                    println!("⚪ No text quads generated, skipping");
                } else {
                    println!("✅ Generated {} text quads", quads.len());
                    if self.set_transform_uniforms(ShaderType::Text).is_ok() {
                        println!("✅ Set text transform uniforms successfully");
                        self.render_text_quads(&quads);
                        println!("✅ Rendered {} text quads", quads.len());
                    } else {
                        println!("❌ Failed to set text transform uniforms");
                    }
                }
            },
            Err(e) => {
                println!("❌ Text rendering failed: {:?}", e);
            }
        }
        
        self.current_transform = old_transform;
    }
    
    fn draw_box_shadow(
        &mut self,
        transform: Affine,
        rect: Rect,
        brush: Color,
        _radius: f64,
        _std_dev: f64,
    ) {
        let color = Self::color_to_array(brush);
        let buffers = self.tessellator.tessellate_rect(rect, color);
        
        let old_transform = self.current_transform;
        self.current_transform = self.current_transform * transform;
        
        if self.set_transform_uniforms(ShaderType::Fill).is_ok() {
            println!("✅ Set transform uniforms successfully for box shadow");
            self.upload_geometry(&buffers.vertices, &buffers.indices);
            self.draw_elements(buffers.indices.len());
            println!("✅ Drew {} elements for box shadow", buffers.indices.len());
        } else {
            println!("❌ Failed to set transform uniforms for box shadow");
        }
        
        self.current_transform = old_transform;
    }
}

impl Drop for GlesScenePainter {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteBuffers(1, &self.ebo);
        }
    }
}
