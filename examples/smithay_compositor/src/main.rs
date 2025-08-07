use anyrender_vello::VelloWindowRenderer;
use blitz_dom::{qual_name, DocumentConfig};
use blitz_html::HtmlDocument;
use blitz_shell::{create_default_event_loop, BlitzApplication, BlitzShellEvent, WindowConfig};
use tracing::debug;

mod smithay_paint_source;
use smithay_paint_source::SmithayPaintSource;

static STYLES: &str = r#"
    body { margin: 0; padding: 20px; font-family: Arial, sans-serif; background: #f0f0f0; }
    #main { max-width: 1200px; margin: 0 auto; }
    #overlay { position: absolute; top: 20px; left: 20px; background: rgba(255,255,255,0.9); 
               padding: 15px; border-radius: 8px; z-index: 10; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
    #canvas-container { width: 100%; height: 600px; border: 2px solid #333; 
                       border-radius: 8px; overflow: hidden; margin-top: 20px; }
    #compositor-canvas { width: 100%; height: 100%; }
    h1 { color: #333; text-align: center; margin-bottom: 20px; }
    h2 { color: #666; margin-top: 0; }
    p { margin: 8px 0; }
    .status { font-weight: bold; color: #2d5a27; }
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    debug!("DEBUG: Starting interactive Smithay-Blitz compositor example");
    
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }

    launch_interactive_compositor()
}

pub fn launch_interactive_compositor() -> Result<(), Box<dyn std::error::Error>> {
    debug!("DEBUG: Initializing interactive compositor with BlitzApplication");
    
    let mut renderer = VelloWindowRenderer::new();
    
    let smithay_paint_source = Box::new(SmithayPaintSource::new()?);
    let paint_source_id = renderer.register_custom_paint_source(smithay_paint_source);
    debug!("DEBUG: Registered SmithayPaintSource with ID: {}", paint_source_id);
    
    let html = HTML.replace("{{STYLES_PLACEHOLDER}}", STYLES);
    let mut doc = HtmlDocument::from_html(&html, DocumentConfig::default());
    
    let canvas_node_id = doc.query_selector("#compositor-canvas").unwrap().unwrap();
    let src_attr = qual_name!("src");
    let src_str = paint_source_id.to_string();
    doc.mutate().set_attribute(canvas_node_id, src_attr, &src_str);
    debug!("DEBUG: Canvas element configured with paint source ID");
    
    let event_loop = create_default_event_loop::<BlitzShellEvent>();
    let mut application = BlitzApplication::new(event_loop.create_proxy());
    let window = WindowConfig::new(Box::new(doc), renderer);
    application.add_window(window);
    
    debug!("DEBUG: Starting BlitzApplication event loop");
    event_loop.run_app(&mut application).unwrap();
    
    Ok(())
}

static HTML: &str = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <style type="text/css">
            {{STYLES_PLACEHOLDER}}
        </style>
    </head>
    <body>
        <main id="main">
            <div id="overlay">
                <h2>Smithay-Blitz Compositor</h2>
                <p>Interactive Wayland compositor using Blitz rendering</p>
                <p>Status: <span class="status">Running</span></p>
                <p>Wayland Socket: <span class="status">wayland-blitz-*</span></p>
                <p>Connected Clients: <span class="status" id="client-count">0</span></p>
                <p>Active Surfaces: <span class="status" id="surface-count">0</span></p>
                <p>Backend: <span class="status">WGPU + Vello</span></p>
                <p><small>Watch the canvas background change as mock surfaces are created!</small></p>
            </div>
            <header>
                <h1>Smithay-Blitz Interactive Compositor</h1>
            </header>
            <div id="canvas-container">
                <canvas id="compositor-canvas"></canvas>
            </div>
        </main>
    </body>
    </html>
"#;
