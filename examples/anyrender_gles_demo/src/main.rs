use anyrender::{PaintScene, WindowRenderer};
use anyrender_gles::GlesWindowRenderer;
use kurbo::{Affine, BezPath, Circle, Point, Rect, RoundedRect, Stroke};
use peniko::{BlendMode, Color, Fill, Font};
use std::sync::Arc;
use tracing::{error, info};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

struct App {
    render_state: RenderState,
    width: u32,
    height: u32,
    test_mode: TestMode,
}

#[derive(Debug, Clone, Copy)]
enum TestMode {
    BasicShapes,
    TextRendering,
    ComplexPaths,
    BlendModes,
    ErrorCases,
}

impl TestMode {
    fn next(self) -> Self {
        match self {
            TestMode::BasicShapes => TestMode::TextRendering,
            TestMode::TextRendering => TestMode::ComplexPaths,
            TestMode::ComplexPaths => TestMode::BlendModes,
            TestMode::BlendModes => TestMode::ErrorCases,
            TestMode::ErrorCases => TestMode::BasicShapes,
        }
    }

    fn description(self) -> &'static str {
        match self {
            TestMode::BasicShapes => "Basic Shapes (rectangles, circles, strokes)",
            TestMode::TextRendering => "Text Rendering (fonts, glyphs, layout)",
            TestMode::ComplexPaths => "Complex Paths (bezier curves, tessellation)",
            TestMode::BlendModes => "Blend Modes (alpha, compositing)",
            TestMode::ErrorCases => "Error Cases (edge cases, validation)",
        }
    }
}

enum RenderState {
    Active {
        window: Arc<Window>,
        renderer: GlesWindowRenderer,
    },
    Suspended(Option<Arc<Window>>),
}

impl App {
    fn new() -> Self {
        Self {
            render_state: RenderState::Suspended(None),
            width: 1024,
            height: 768,
            test_mode: TestMode::BasicShapes,
        }
    }

    fn request_redraw(&mut self) {
        let window = match &self.render_state {
            RenderState::Active { window, renderer } => {
                if renderer.is_active() {
                    Some(window)
                } else {
                    None
                }
            }
            RenderState::Suspended(_) => None,
        };

        if let Some(window) = window {
            window.request_redraw();
        }
    }

    fn draw_basic_shapes<T: PaintScene>(scene: &mut T) {
        info!("🎨 Drawing basic shapes test");
        
        scene.fill(
            Fill::NonZero,
            Affine::translate((50.0, 50.0)),
            Color::from_rgb8(255, 100, 100),
            None,
            &Rect::new(0.0, 0.0, 100.0, 80.0),
        );

        scene.stroke(
            &Stroke::new(3.0),
            Affine::translate((200.0, 50.0)),
            Color::from_rgb8(100, 255, 100),
            None,
            &Rect::new(0.0, 0.0, 100.0, 80.0),
        );

        scene.fill(
            Fill::NonZero,
            Affine::translate((350.0, 90.0)),
            Color::from_rgb8(100, 100, 255),
            None,
            &Circle::new(Point::ORIGIN, 40.0),
        );

        scene.fill(
            Fill::NonZero,
            Affine::translate((50.0, 200.0)),
            Color::from_rgb8(255, 255, 100),
            None,
            &RoundedRect::new(0.0, 0.0, 120.0, 60.0, 15.0),
        );

        scene.draw_box_shadow(
            Affine::translate((250.0, 200.0)),
            Rect::new(0.0, 0.0, 100.0, 60.0),
            Color::from_rgba8(0, 0, 0, 128),
            10.0,
            5.0,
        );
    }

    fn draw_text_rendering<T: PaintScene>(scene: &mut T) {
        info!("🎨 Drawing text rendering test");
        
        let glyphs = vec![
            anyrender::types::Glyph {
                id: 65,
                x: 0.0,
                y: 0.0,
            },
            anyrender::types::Glyph {
                id: 66,
                x: 20.0,
                y: 0.0,
            },
            anyrender::types::Glyph {
                id: 67,
                x: 40.0,
                y: 0.0,
            },
        ];

        let font_data = vec![0u8; 100];
        let font = Font::new(peniko::Blob::new(Arc::new(font_data)), 0);
        scene.draw_glyphs(
            &font,
            24.0,
            false,
            &[],
            Fill::NonZero,
            Color::from_rgb8(255, 255, 255),
            1.0,
            Affine::translate((50.0, 300.0)),
            None,
            glyphs.into_iter(),
        );
    }

    fn draw_complex_paths<T: PaintScene>(scene: &mut T) {
        info!("🎨 Drawing complex paths test");
        
        let mut path = BezPath::new();
        path.move_to((50.0, 400.0));
        path.curve_to((100.0, 350.0), (150.0, 450.0), (200.0, 400.0));
        path.quad_to((250.0, 380.0), (300.0, 420.0));
        path.line_to((350.0, 400.0));
        path.close_path();

        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(255, 150, 50),
            None,
            &path,
        );

        scene.stroke(
            &Stroke::new(2.0),
            Affine::translate((0.0, 100.0)),
            Color::from_rgb8(50, 150, 255),
            None,
            &path,
        );
    }

    fn draw_blend_modes<T: PaintScene>(scene: &mut T) {
        info!("🎨 Drawing blend modes test");
        
        scene.push_layer(
            BlendMode::default(),
            0.8,
            Affine::IDENTITY,
            &Rect::new(400.0, 50.0, 600.0, 250.0),
        );

        scene.fill(
            Fill::NonZero,
            Affine::translate((420.0, 70.0)),
            Color::from_rgba8(255, 0, 0, 180),
            None,
            &Circle::new(Point::ORIGIN, 30.0),
        );

        scene.fill(
            Fill::NonZero,
            Affine::translate((460.0, 90.0)),
            Color::from_rgba8(0, 255, 0, 180),
            None,
            &Circle::new(Point::ORIGIN, 30.0),
        );

        scene.fill(
            Fill::NonZero,
            Affine::translate((440.0, 130.0)),
            Color::from_rgba8(0, 0, 255, 180),
            None,
            &Circle::new(Point::ORIGIN, 30.0),
        );

        scene.pop_layer();
    }

    fn draw_error_cases<T: PaintScene>(scene: &mut T) {
        info!("🎨 Drawing error cases test");
        
        scene.fill(
            Fill::NonZero,
            Affine::translate((50.0, 600.0)),
            Color::from_rgb8(255, 255, 255),
            None,
            &Rect::new(0.0, 0.0, 0.1, 0.1),
        );

        scene.fill(
            Fill::NonZero,
            Affine::translate((100.0, 600.0)),
            Color::from_rgb8(200, 200, 200),
            None,
            &Rect::new(0.0, 0.0, 10000.0, 10000.0),
        );

        let mut degenerate_path = BezPath::new();
        degenerate_path.move_to((200.0, 600.0));
        degenerate_path.line_to((200.0, 600.0));
        degenerate_path.close_path();

        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(100, 100, 100),
            None,
            &degenerate_path,
        );

        let mut problematic_path = BezPath::new();
        problematic_path.move_to((300.0, 600.0));
        problematic_path.line_to((350.0, 620.0));
        
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(150, 150, 150),
            None,
            &problematic_path,
        );
    }

    fn draw_scene<T: PaintScene>(scene: &mut T, test_mode: TestMode) {
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(30, 30, 30),
            None,
            &Rect::new(0.0, 0.0, 1024.0, 768.0),
        );

        info!("🎯 Running test mode: {}", test_mode.description());

        match test_mode {
            TestMode::BasicShapes => Self::draw_basic_shapes(scene),
            TestMode::TextRendering => Self::draw_text_rendering(scene),
            TestMode::ComplexPaths => Self::draw_complex_paths(scene),
            TestMode::BlendModes => Self::draw_blend_modes(scene),
            TestMode::ErrorCases => Self::draw_error_cases(scene),
        }
    }

    fn resume_with_gles(&mut self, event_loop: &ActiveEventLoop) {
        let window = match &self.render_state {
            RenderState::Active { window, .. } => Some(window.clone()),
            RenderState::Suspended(cached_window) => cached_window.clone(),
        };

        let window = window.unwrap_or_else(|| {
            let attr = Window::default_attributes()
                .with_inner_size(winit::dpi::PhysicalSize::new(self.width, self.height))
                .with_resizable(true)
                .with_title("anyrender_gles Comprehensive Test Demo")
                .with_visible(true)
                .with_active(true);
            Arc::new(event_loop.create_window(attr).unwrap())
        });

        info!("🚀 Initializing GLES renderer...");
        
        let mut renderer = GlesWindowRenderer::new();
        info!("✅ GLES renderer created successfully");
        renderer.resume(window.clone(), self.width, self.height);
        
        self.render_state = RenderState::Active { window, renderer };
        self.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        info!("🔄 Application suspended");
        if let RenderState::Active { window, .. } = &self.render_state {
            self.render_state = RenderState::Suspended(Some(window.clone()));
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("🔄 Application resumed");
        self.resume_with_gles(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let RenderState::Active { window, renderer } = &mut self.render_state else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                info!("🛑 Close requested");
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                info!("📏 Window resized to {}x{}", physical_size.width, physical_size.height);
                self.width = physical_size.width;
                self.height = physical_size.height;
                renderer.set_size(self.width, self.height);
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                info!("🎨 Redraw requested for test mode: {}", self.test_mode.description());
                
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    renderer.render(|scene| {
                        Self::draw_scene(scene, self.test_mode);
                    });
                })) {
                    Ok(_) => {
                        info!("✅ Render completed successfully");
                    }
                    Err(e) => {
                        error!("❌ Render failed with panic: {:?}", e);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Space),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.test_mode = self.test_mode.next();
                info!("🔄 Switched to test mode: {}", self.test_mode.description());
                self.request_redraw();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                info!("🛑 Escape pressed, exiting");
                event_loop.exit();
            }
            _ => {}
        }
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Starting anyrender_gles comprehensive test demo");
    info!("📋 Controls:");
    info!("   - SPACE: Switch between test modes");
    info!("   - ESC: Exit application");

    let mut app = App::new();
    let event_loop = EventLoop::new()?;
    
    info!("🔄 Starting event loop...");
    event_loop.run_app(&mut app)?;
    
    info!("👋 Demo completed");
    Ok(())
}
