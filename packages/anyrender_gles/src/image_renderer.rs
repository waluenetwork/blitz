use crate::scene::GlesScenePainter;
use anyrender::ImageRenderer;

pub struct GlesImageRenderer {
    width: u32,
    height: u32,
}

impl ImageRenderer for GlesImageRenderer {
    type ScenePainter<'a> = GlesScenePainter where Self: 'a;

    fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    fn render<F: FnOnce(&mut Self::ScenePainter<'_>)>(&mut self, draw_fn: F, buffer: &mut Vec<u8>) {
        let mut scene_painter = match GlesScenePainter::new(self.width, self.height) {
            Ok(painter) => painter,
            Err(e) => {
                tracing::error!("Failed to create scene painter for image rendering: {}", e);
                return;
            }
        };

        draw_fn(&mut scene_painter);

        buffer.clear();
        buffer.resize((self.width * self.height * 4) as usize, 0);
        
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = ((y * self.width + x) * 4) as usize;
                buffer[idx] = (x * 255 / self.width) as u8;
                buffer[idx + 1] = (y * 255 / self.height) as u8;
                buffer[idx + 2] = 128;
                buffer[idx + 3] = 255;
            }
        }
    }
}
