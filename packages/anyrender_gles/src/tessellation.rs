use anyhow::Result;
use kurbo::{BezPath, PathEl, Rect};
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};
use lyon::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl From<FillVertex<'_>> for Vertex {
    fn from(vertex: FillVertex) -> Self {
        Self {
            position: vertex.position().to_array(),
            color: [1.0, 1.0, 1.0, 1.0], // Default white
        }
    }
}

impl From<StrokeVertex<'_, '_>> for Vertex {
    fn from(vertex: StrokeVertex) -> Self {
        Self {
            position: vertex.position().to_array(),
            color: [1.0, 1.0, 1.0, 1.0], // Default white
        }
    }
}

pub struct GlesTessellator {
    fill_tessellator: FillTessellator,
    stroke_tessellator: StrokeTessellator,
}

impl GlesTessellator {
    pub fn new() -> Self {
        Self {
            fill_tessellator: FillTessellator::new(),
            stroke_tessellator: StrokeTessellator::new(),
        }
    }

    pub fn tessellate_fill(&mut self, path: &BezPath, color: [f32; 4]) -> Result<VertexBuffers<Vertex, u32>> {
        let lyon_path = self.convert_bezpath_to_lyon(path)?;
        let options = FillOptions::default();

        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
        let mut builder = BuffersBuilder::new(&mut buffers, |vertex: FillVertex| {
            Vertex {
                position: vertex.position().to_array(),
                color,
            }
        });

        self.fill_tessellator.tessellate_path(&lyon_path, &options, &mut builder)?;

        Ok(buffers)
    }

    pub fn tessellate_stroke(&mut self, path: &BezPath, color: [f32; 4], width: f32) -> Result<VertexBuffers<Vertex, u32>> {
        let lyon_path = self.convert_bezpath_to_lyon(path)?;
        let options = StrokeOptions::default().with_line_width(width);

        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
        let mut builder = BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex| {
            Vertex {
                position: vertex.position().to_array(),
                color,
            }
        });

        self.stroke_tessellator.tessellate_path(&lyon_path, &options, &mut builder)?;

        Ok(buffers)
    }

    pub fn tessellate_rect(&self, rect: Rect, color: [f32; 4]) -> VertexBuffers<Vertex, u32> {
        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();

        let x0 = rect.x0 as f32;
        let y0 = rect.y0 as f32;
        let x1 = rect.x1 as f32;
        let y1 = rect.y1 as f32;

        buffers.vertices.extend_from_slice(&[
            Vertex { position: [x0, y0], color },
            Vertex { position: [x1, y0], color },
            Vertex { position: [x1, y1], color },
            Vertex { position: [x0, y1], color },
        ]);

        buffers.indices.extend_from_slice(&[0, 1, 2, 2, 3, 0]);

        buffers
    }

    fn convert_bezpath_to_lyon(&self, bezpath: &BezPath) -> Result<Path> {
        let mut builder = Path::builder();
        let mut path_started = false;

        for el in bezpath.elements() {
            match el {
                PathEl::MoveTo(p) => {
                    if path_started {
                        builder.end(false);
                    }
                    builder.begin(lyon::math::Point::new(p.x as f32, p.y as f32));
                    path_started = true;
                }
                PathEl::LineTo(p) => {
                    builder.line_to(lyon::math::Point::new(p.x as f32, p.y as f32));
                }
                PathEl::QuadTo(p1, p2) => {
                    builder.quadratic_bezier_to(
                        lyon::math::Point::new(p1.x as f32, p1.y as f32),
                        lyon::math::Point::new(p2.x as f32, p2.y as f32),
                    );
                }
                PathEl::CurveTo(p1, p2, p3) => {
                    builder.cubic_bezier_to(
                        lyon::math::Point::new(p1.x as f32, p1.y as f32),
                        lyon::math::Point::new(p2.x as f32, p2.y as f32),
                        lyon::math::Point::new(p3.x as f32, p3.y as f32),
                    );
                }
                PathEl::ClosePath => {
                    builder.close();
                    path_started = false;
                }
            }
        }

        if path_started {
            builder.end(false);
        }

        Ok(builder.build())
    }
}

impl Default for GlesTessellator {
    fn default() -> Self {
        Self::new()
    }
}
