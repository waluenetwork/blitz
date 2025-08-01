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

    let egui_id = use_egui(move |ctx| {
        use std::sync::Mutex;
        use std::sync::OnceLock;
        
        static TEXT_INPUT: OnceLock<Mutex<String>> = OnceLock::new();
        static SHOW_CONTENT: OnceLock<Mutex<bool>> = OnceLock::new();
        static BUTTON_CLICKS: OnceLock<Mutex<i32>> = OnceLock::new();
        static SLIDER_VALUE: OnceLock<Mutex<i32>> = OnceLock::new();
        static CHECKBOX_STATE: OnceLock<Mutex<bool>> = OnceLock::new();
        
        let text_input = TEXT_INPUT.get_or_init(|| Mutex::new(String::new()));
        let show_content = SHOW_CONTENT.get_or_init(|| Mutex::new(true));
        let button_clicks = BUTTON_CLICKS.get_or_init(|| Mutex::new(0));
        let slider_value = SLIDER_VALUE.get_or_init(|| Mutex::new(50));
        let checkbox_state = CHECKBOX_STATE.get_or_init(|| Mutex::new(false));
        
        egui::Window::new("Interactive Egui Demo")
            .default_size([500.0, 400.0])
            .default_pos([150.0, 100.0])
            .resizable(true)
            .movable(true)
            .show(ctx, |ui| {
                ui.heading("Hello from Interactive Egui!");
                ui.separator();
                
                ui.label("Interactive Text Input:");
                if let Ok(mut text) = text_input.try_lock() {
                    let response = ui.text_edit_singleline(&mut *text);
                    if response.changed() {
                        
                    }
                    ui.label(format!("You typed: {}", *text));
                } else {
                    ui.label("Failed to lock text input mutex");
                }
                ui.separator();
                
                if ui.button("Toggle Content").clicked() {
                    if let (Ok(mut show), Ok(mut clicks)) = (show_content.try_lock(), button_clicks.try_lock()) {
                        *show = !*show;
                        *clicks += 1;
                        
                    } else {
                        
                    }
                }
                
                if let Ok(show) = show_content.try_lock() {
                    if *show {
                        ui.label("🎉 This content is visible!");
                        ui.label("Click the button above to hide me.");
                    } else {
                        ui.label("Content is hidden. Click the button to show it!");
                    }
                }
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    ui.label("Interactive Slider:");
                    if let Ok(mut value) = slider_value.try_lock() {
                        let response = ui.add(egui::Slider::new(&mut *value, 0..=100));
                        if response.changed() {
                            
                        }
                    } else {
                        ui.label("Failed to lock slider value mutex");
                    }
                });
                
                ui.separator();
                
                if let Ok(mut checked) = checkbox_state.try_lock() {
                    let response = ui.checkbox(&mut *checked, "Interactive Checkbox");
                    if response.changed() {
                        
                    }
                    
                    if *checked {
                        ui.label("✓ Checkbox is checked!");
                    } else {
                        ui.label("☐ Checkbox is unchecked");
                    }
                } else {
                    ui.label("Failed to lock checkbox state mutex");
                }
                
                if let Ok(clicks) = button_clicks.try_lock() {
                    ui.label(format!("Button clicked {} times", *clicks));
                }
                ui.label("Interactive egui demo ready for testing!");
            });
    });

    
    rsx! {
        canvas {
            class: "egui-canvas",
            tabindex: "0",
            width: "800",
            height: "600",
            "src": egui_id
        }
    }
}

fn use_egui<F>(ui_fn: F) -> u64 
where 
    F: Fn(&egui::Context) + Send + 'static
{
    use blitz_egui::EguiPaintSource;
    
    let id = use_wgpu(move || {
        EguiPaintSource::with_ui(ui_fn)
    });
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
    display: block;
}
"#;
