use dioxus::prelude::*;
use mini_dxn::use_wgpu;
use tracing::debug;

mod smithay_paint_source;
use smithay_paint_source::SmithayPaintSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    debug!("DEBUG: Starting interactive Smithay-Blitz compositor example");
    
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }

    mini_dxn::launch(app);
    Ok(())
}

fn app() -> Element {
    rsx! {
        style { {CSS} }
        div {
            class: "container",
            div {
                class: "overlay",
                h2 { "Smithay-Blitz Compositor" }
                p { "Interactive Wayland compositor using Blitz rendering" }
                p { "Status: " span { class: "status", "Running" } }
                p { "Wayland Socket: " span { class: "status", "wayland-blitz-*" } }
                p { "Connected Clients: " span { class: "status", "0" } }
                p { "Active Surfaces: " span { class: "status", "0" } }
                p { "Backend: " span { class: "status", "WGPU + Vello" } }
                p { small { "Real Wayland surfaces will appear when clients connect!" } }
            }
            header {
                h1 { "Smithay-Blitz Interactive Compositor" }
            }
            div {
                class: "canvas-container",
                SmithayCompositor {}
            }
        }
    }
}

#[component]
fn SmithayCompositor() -> Element {
    let smithay_paint_source = SmithayPaintSource::new().expect("Failed to create SmithayPaintSource");
    let paint_source_id = use_wgpu(move || smithay_paint_source);

    rsx! {
        canvas {
            class: "compositor-canvas",
            tabindex: "0",
            width: "800",
            height: "600",
            "src": paint_source_id
        }
    }
}

const CSS: &str = r#"
.container {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 20px;
    font-family: Arial, sans-serif;
    background: #f0f0f0;
    min-height: 100vh;
}

.overlay {
    position: absolute;
    top: 20px;
    left: 20px;
    background: rgba(255,255,255,0.9);
    padding: 15px;
    border-radius: 8px;
    z-index: 10;
    box-shadow: 0 2px 10px rgba(0,0,0,0.1);
}

.canvas-container {
    width: 800px;
    height: 600px;
    border: 2px solid #333;
    border-radius: 8px;
    overflow: hidden;
    margin-top: 20px;
}

.compositor-canvas {
    width: 100%;
    height: 100%;
    display: block;
}

h1 {
    color: #333;
    text-align: center;
    margin-bottom: 20px;
}

h2 {
    color: #666;
    margin-top: 0;
}

p {
    margin: 8px 0;
}

.status {
    font-weight: bold;
    color: #2d5a27;
}
"#;
