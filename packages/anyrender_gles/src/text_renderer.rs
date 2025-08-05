use anyhow::Result;
use fontdue::{Font, FontSettings};
use gl::types::*;
use kurbo::Point;
use rustc_hash::FxHashMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct GlyphTexture {
    pub texture_id: GLuint,
    pub width: u32,
    pub height: u32,
    pub advance_width: f32,
    pub bearing_x: i32,
    pub bearing_y: i32,
}

pub struct GlesTextRenderer {
    font_cache: HashMap<String, Font>,
    glyph_cache: FxHashMap<(char, u32), GlyphTexture>,
    default_font: Option<Font>,
}

impl GlesTextRenderer {
    pub fn new() -> Result<Self> {
        let mut renderer = Self {
            font_cache: HashMap::new(),
            glyph_cache: FxHashMap::default(),
            default_font: None,
        };
        
        renderer.load_default_font()?;
        
        Ok(renderer)
    }
    
    fn load_default_font(&mut self) -> Result<()> {
        let font_data: &[u8] = &[0u8; 1024][..];
        let font = Font::from_bytes(font_data, FontSettings::default())
            .unwrap_or_else(|_| {
                Font::from_bytes(&[0u8; 1024][..], FontSettings::default())
                    .expect("Failed to create minimal font")
            });
        
        self.default_font = Some(font.clone());
        self.font_cache.insert("default".to_string(), font);
        
        Ok(())
    }
    
    pub fn render_text(&mut self, text: &str, font_size: f32, position: Point, color: [f32; 4]) -> Result<Vec<TextQuad>> {
        let mut quads = Vec::new();
        
        let mut cursor_x = position.x as f32;
        let cursor_y = position.y as f32;
        
        for ch in text.chars() {
            let glyph_texture = self.get_or_rasterize_glyph(ch, font_size as u32)?;
            
            let quad = TextQuad {
                position: [
                    cursor_x + glyph_texture.bearing_x as f32,
                    cursor_y - glyph_texture.bearing_y as f32,
                ],
                size: [glyph_texture.width as f32, glyph_texture.height as f32],
                texture_id: glyph_texture.texture_id,
                color,
            };
            
            quads.push(quad);
            cursor_x += glyph_texture.advance_width;
        }
        
        Ok(quads)
    }
    
    fn get_font(&self, name: &str) -> Result<&Font> {
        self.font_cache.get(name)
            .ok_or_else(|| anyhow::anyhow!("Font '{}' not found", name))
    }
    
    fn get_or_rasterize_glyph(&mut self, ch: char, size: u32) -> Result<&GlyphTexture> {
        let key = (ch, size);
        
        if !self.glyph_cache.contains_key(&key) {
            let font = self.get_font("default")?;
            let (metrics, bitmap) = font.rasterize(ch, size as f32);
            
            let texture_id = self.upload_glyph_bitmap(&bitmap, metrics.width, metrics.height)?;
            
            let glyph_texture = GlyphTexture {
                texture_id,
                width: metrics.width as u32,
                height: metrics.height as u32,
                advance_width: metrics.advance_width,
                bearing_x: metrics.xmin,
                bearing_y: metrics.ymin,
            };
            
            self.glyph_cache.insert(key, glyph_texture);
        }
        
        Ok(&self.glyph_cache[&key])
    }
    
    fn upload_glyph_bitmap(&self, bitmap: &[u8], width: usize, height: usize) -> Result<GLuint> {
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
                gl::R8 as i32,
                width as i32,
                height as i32,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                bitmap.as_ptr() as *const _,
            );
            
            gl::BindTexture(gl::TEXTURE_2D, 0);
            
            Ok(texture_id)
        }
    }
}

impl Drop for GlesTextRenderer {
    fn drop(&mut self) {
        unsafe {
            for glyph in self.glyph_cache.values() {
                gl::DeleteTextures(1, &glyph.texture_id);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextQuad {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub texture_id: GLuint,
    pub color: [f32; 4],
}

impl Default for GlesTextRenderer {
    fn default() -> Self {
        Self::new().expect("Failed to create default text renderer")
    }
}
