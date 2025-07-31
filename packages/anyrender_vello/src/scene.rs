use anyrender::{CustomPaint, NormalizedCoord, Paint, PaintScene};
use kurbo::{Affine, Rect, Shape, Stroke};
use peniko::{BlendMode, BrushRef, Color, Fill, Font, StyleRef};
use rustc_hash::FxHashMap;
use vello::Renderer as VelloRenderer;

use crate::{CustomPaintSource, custom_paint_source::CustomPaintCtx};

pub struct VelloScenePainter<'r> {
    pub renderer: &'r mut VelloRenderer,
    pub custom_paint_sources: &'r mut FxHashMap<u64, Box<dyn CustomPaintSource>>,
    pub inner: vello::Scene,
}

impl VelloScenePainter<'_> {
    fn render_custom_source(&mut self, custom_paint: CustomPaint) -> Option<peniko::Image> {
        let CustomPaint {
            source_id,
            width,
            height,
            scale,
        } = custom_paint;

        println!("DEBUG: render_custom_source called with source_id: {}, dimensions: {}x{}", 
                 source_id, width, height);
        println!("DEBUG: Available custom paint sources: {:?}", self.custom_paint_sources.keys().collect::<Vec<_>>());

        let source = self.custom_paint_sources.get_mut(&source_id);
        if source.is_none() {
            println!("DEBUG: No custom paint source found for ID: {}", source_id);
            return None;
        }
        
        let source = source.unwrap();
        println!("DEBUG: Found custom paint source for ID: {}", source_id);
        
        let ctx = CustomPaintCtx::new(self.renderer);
        let texture_handle = source.render(ctx, width, height, scale);
        
        match texture_handle {
            Some(handle) => {
                println!("DEBUG: Custom paint source render completed successfully");
                Some(handle.dummy_image())
            }
            None => {
                println!("DEBUG: Custom paint source render returned None");
                None
            }
        }
    }
}

impl VelloScenePainter<'_> {
    pub fn finish(self) -> vello::Scene {
        self.inner
    }
}

impl PaintScene for VelloScenePainter<'_> {
    fn reset(&mut self) {
        self.inner.reset();
    }

    fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        clip: &impl Shape,
    ) {
        self.inner.push_layer(blend, alpha, transform, clip);
    }

    fn pop_layer(&mut self) {
        self.inner.pop_layer();
    }

    fn stroke<'a>(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: impl Into<BrushRef<'a>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        self.inner
            .stroke(style, transform, brush, brush_transform, shape);
    }

    fn fill<'a>(
        &mut self,
        style: Fill,
        transform: Affine,
        paint: impl Into<Paint<'a>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let paint: Paint<'_> = paint.into();

        let dummy_image: peniko::Image;
        let brush_ref = match paint {
            Paint::Solid(color) => {
                println!("DEBUG: fill called with Solid color");
                BrushRef::Solid(color)
            }
            Paint::Gradient(gradient) => {
                println!("DEBUG: fill called with Gradient");
                BrushRef::Gradient(gradient)
            }
            Paint::Image(image) => {
                println!("DEBUG: fill called with Image");
                BrushRef::Image(image)
            }
            Paint::Custom(custom_paint) => {
                println!("DEBUG: fill called with Custom paint");
                let Ok(custom_paint) = custom_paint.downcast::<CustomPaint>() else {
                    println!("DEBUG: Failed to downcast custom paint");
                    return;
                };
                println!("DEBUG: Successfully downcast CustomPaint, calling render_custom_source");
                let Some(image) = self.render_custom_source(*custom_paint) else {
                    println!("DEBUG: render_custom_source returned None, skipping fill");
                    return;
                };
                dummy_image = image;
                BrushRef::Image(&dummy_image)
            }
        };

        println!("DEBUG: Calling inner.fill with brush_ref");
        self.inner
            .fill(style, transform, brush_ref, brush_transform, shape);
    }

    fn draw_glyphs<'a, 's: 'a>(
        &'a mut self,
        font: &'a Font,
        font_size: f32,
        hint: bool,
        normalized_coords: &'a [NormalizedCoord],
        style: impl Into<StyleRef<'a>>,
        brush: impl Into<BrushRef<'a>>,
        brush_alpha: f32,
        transform: Affine,
        glyph_transform: Option<Affine>,
        glyphs: impl Iterator<Item = anyrender::Glyph>,
    ) {
        self.inner
            .draw_glyphs(font)
            .font_size(font_size)
            .hint(hint)
            .normalized_coords(normalized_coords)
            .brush(brush)
            .brush_alpha(brush_alpha)
            .transform(transform)
            .glyph_transform(glyph_transform)
            .draw(
                style,
                glyphs.map(|g: anyrender::Glyph| vello::Glyph {
                    id: g.id,
                    x: g.x,
                    y: g.y,
                }),
            );
    }

    fn draw_box_shadow(
        &mut self,
        transform: Affine,
        rect: Rect,
        brush: Color,
        radius: f64,
        std_dev: f64,
    ) {
        self.inner
            .draw_blurred_rounded_rect(transform, rect, brush, radius, std_dev);
    }
}
