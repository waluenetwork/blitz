use dioxus::prelude::*;
use mini_dxn::use_wgpu;

fn main() {
    tracing_subscriber::fmt::init();
    mini_dxn::launch(app);
}

fn app() -> Element {
    rsx! {
        style { {CSS} }
        div {
            class: "container",
            h1 { "Blitz + Egui Integration Demo" }
            p { "This demonstrates egui running inside Blitz using CustomPaintSource." }
            div {
                class: "canvas-container",
                EguiDemo {}
            }
        }
    }
}

#[component]
fn EguiDemo() -> Element {
    let egui_id = use_egui(|ctx| {
        egui::Window::new("Interactive Egui Demo")
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                ui.heading("Hello from Interactive Egui!");
                ui.separator();
                
                ui.label("This is egui running inside Blitz with basic interactivity!");
                
                if ui.button("Click me!").clicked() {
                    println!("Button clicked in egui!");
                }
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    ui.label("Interactive Slider:");
                    let mut slider_value = 50;
                    ui.add(egui::Slider::new(&mut slider_value, 0..=100));
                });
                
                ui.separator();
                
                let mut checkbox_state = false;
                ui.checkbox(&mut checkbox_state, "Interactive Checkbox");
                
                ui.separator();
                
                ui.label("Interactive Text Input:");
                let mut text_content = String::from("Type here...");
                ui.text_edit_singleline(&mut text_content);
            });
    });

    println!("DEBUG: EguiDemo component rendering canvas with src={}", egui_id);
    
    rsx! {
        canvas {
            class: "egui-canvas",
            "src": egui_id
        }
    }
}

fn use_egui<F>(ui_fn: F) -> u64 
where 
    F: Fn(&egui::Context) + Send + 'static
{
    use blitz_egui::EguiPaintSource;
    
    println!("DEBUG: use_egui called, creating EguiPaintSource");
    let id = use_wgpu(move || {
        println!("DEBUG: use_egui creating EguiPaintSource::with_ui");
        EguiPaintSource::with_ui(ui_fn)
    });
    println!("DEBUG: use_egui completed, returned ID: {}", id);
    id
}

const CSS: &str = r#"
.container {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 20px;
    font-family: sans-serif;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    min-height: 100vh;
}

.canvas-container {
    width: 800px;
    height: 600px;
    border: 2px solid #333;
    margin: 20px;
    background: rgba(255, 255, 255, 0.1);
}

.egui-canvas {
    width: 100%;
    height: 100%;
}
"#;
