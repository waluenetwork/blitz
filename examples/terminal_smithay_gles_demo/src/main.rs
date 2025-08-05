//! 

use anyhow::Result;
use blitz_smithay_gles::BlitzSmithayIntegration;
use dioxus::prelude::*;
use mini_dxn::launch;
use std::sync::{Arc, Mutex};
use tracing::{info, error};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting Terminal Smithay GLES Demo");

    launch(app);
    Ok(())
}

fn app() -> Element {
    let mut integration = use_signal(|| None::<Arc<Mutex<BlitzSmithayIntegration>>>);
    let mut terminals = use_signal(|| Vec::<u64>::new());
    let mut status_message = use_signal(|| "Ready".to_string());
    let mut performance_stats = use_signal(|| PerformanceStats::default());

    use_effect(move || {
        if integration.read().is_none() {
            match initialize_integration() {
                Ok(int) => {
                    integration.set(Some(Arc::new(Mutex::new(int))));
                    status_message.set("Integration initialized successfully".to_string());
                }
                Err(e) => {
                    error!("Failed to initialize integration: {}", e);
                    status_message.set(format!("Error: {}", e));
                }
            }
        }
    });

    rsx! {
        div {
            style: "font-family: 'Segoe UI', sans-serif; padding: 20px; background: #1e1e1e; color: #ffffff; min-height: 100vh;",
            
            header {
                style: "margin-bottom: 30px; border-bottom: 2px solid #333; padding-bottom: 20px;",
                h1 {
                    style: "color: #4CAF50; margin: 0; font-size: 2.5em;",
                    "🖥️ Terminal Smithay GLES Demo"
                }
                p {
                    style: "color: #888; margin: 10px 0 0 0; font-size: 1.1em;",
                    "Interactive Wayland compositor with Blitz GLES rendering"
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 30px; margin-bottom: 30px;",
                
                div {
                    style: "background: #2d2d2d; padding: 20px; border-radius: 8px; border: 1px solid #444;",
                    h2 {
                        style: "color: #4CAF50; margin-top: 0; font-size: 1.5em;",
                        "🎮 Compositor Controls"
                    }
                    
                    div {
                        style: "margin-bottom: 15px;",
                        button {
                            style: "background: #4CAF50; color: white; border: none; padding: 12px 20px; border-radius: 4px; cursor: pointer; margin-right: 10px; font-size: 1em;",
                            onclick: {
                                let integration = integration.clone();
                                let terminals = terminals.clone();
                                let status_message = status_message.clone();
                                move |_| spawn_terminal(integration, terminals, status_message, "default")
                            },
                            "🚀 Spawn weston-terminal"
                        }
                        button {
                            style: "background: #2196F3; color: white; border: none; padding: 12px 20px; border-radius: 4px; cursor: pointer; margin-right: 10px; font-size: 1em;",
                            onclick: {
                                let integration = integration.clone();
                                let terminals = terminals.clone();
                                let status_message = status_message.clone();
                                move |_| spawn_terminal(integration, terminals, status_message, "htop")
                            },
                            "📊 Launch htop"
                        }
                    }
                    
                    div {
                        style: "margin-bottom: 15px;",
                        button {
                            style: "background: #FF9800; color: white; border: none; padding: 12px 20px; border-radius: 4px; cursor: pointer; margin-right: 10px; font-size: 1em;",
                            onclick: {
                                let integration = integration.clone();
                                let terminals = terminals.clone();
                                let status_message = status_message.clone();
                                move |_| spawn_terminal(integration, terminals, status_message, "vim")
                            },
                            "✏️ Open Vim Editor"
                        }
                        button {
                            style: "background: #9C27B0; color: white; border: none; padding: 12px 20px; border-radius: 4px; cursor: pointer; font-size: 1em;",
                            onclick: {
                                let integration = integration.clone();
                                let terminals = terminals.clone();
                                let status_message = status_message.clone();
                                move |_| spawn_terminal(integration, terminals, status_message, "python3")
                            },
                            "🐍 Python REPL"
                        }
                    }
                    
                    if !terminals.read().is_empty() {
                        div {
                            style: "margin-top: 20px;",
                            button {
                                style: "background: #f44336; color: white; border: none; padding: 10px 16px; border-radius: 4px; cursor: pointer; font-size: 0.9em;",
                                onclick: {
                                    let integration = integration.clone();
                                    let terminals = terminals.clone();
                                    let status_message = status_message.clone();
                                    move |_| kill_all_terminals(integration, terminals, status_message)
                                },
                                "❌ Kill All Terminals"
                            }
                        }
                    }
                }

                div {
                    style: "background: #2d2d2d; padding: 20px; border-radius: 8px; border: 1px solid #444;",
                    h2 {
                        style: "color: #4CAF50; margin-top: 0; font-size: 1.5em;",
                        "📊 System Status"
                    }
                    
                    div {
                        style: "margin-bottom: 15px;",
                        strong { "Status: " }
                        span {
                            style: if status_message.read().contains("Error") { "color: #f44336;" } else { "color: #4CAF50;" },
                            "{status_message}"
                        }
                    }
                    
                    div {
                        style: "margin-bottom: 15px;",
                        strong { "Active Terminals: " }
                        span { style: "color: #2196F3;", "{terminals.read().len()}" }
                    }
                    
                    div {
                        style: "margin-bottom: 15px;",
                        strong { "Rendering Backend: " }
                        span { style: "color: #FF9800;", "OpenGL ES 3.0" }
                    }
                    
                    div {
                        style: "margin-bottom: 15px;",
                        strong { "Compositor: " }
                        span { style: "color: #9C27B0;", "Smithay" }
                    }
                }
            }

            if !terminals.read().is_empty() {
                div {
                    style: "background: #2d2d2d; padding: 20px; border-radius: 8px; border: 1px solid #444; margin-bottom: 30px;",
                    h2 {
                        style: "color: #4CAF50; margin-top: 0; font-size: 1.5em;",
                        "🖥️ Active Terminals"
                    }
                    
                    div {
                        style: "display: grid; gap: 10px;",
                        for terminal_id in terminals.read().iter() {
                            div {
                                key: "{terminal_id}",
                                style: "background: #1e1e1e; padding: 15px; border-radius: 4px; border: 1px solid #555; display: flex; justify-content: space-between; align-items: center;",
                                div {
                                    span {
                                        style: "color: #4CAF50; font-weight: bold; margin-right: 10px;",
                                        "Terminal #{terminal_id}"
                                    }
                                    span {
                                        style: "color: #888;",
                                        "Running in Wayland surface"
                                    }
                                }
                                button {
                                    style: "background: #f44336; color: white; border: none; padding: 8px 12px; border-radius: 4px; cursor: pointer; font-size: 0.8em;",
                                    onclick: {
                                        let integration = integration.clone();
                                        let terminals = terminals.clone();
                                        let status_message = status_message.clone();
                                        let terminal_id = *terminal_id;
                                        move |_| kill_terminal(integration, terminals, status_message, terminal_id)
                                    },
                                    "❌ Kill"
                                }
                            }
                        }
                    }
                }
            }

            div {
                style: "background: #2d2d2d; padding: 20px; border-radius: 8px; border: 1px solid #444;",
                h2 {
                    style: "color: #4CAF50; margin-top: 0; font-size: 1.5em;",
                    "⚡ Performance Monitor"
                }
                
                div {
                    style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px;",
                    
                    div {
                        style: "background: #1e1e1e; padding: 15px; border-radius: 4px; border: 1px solid #555;",
                        div {
                            style: "color: #888; font-size: 0.9em; margin-bottom: 5px;",
                            "Frame Rate"
                        }
                        div {
                            style: "color: #4CAF50; font-size: 1.5em; font-weight: bold;",
                            "{performance_stats.read().fps} FPS"
                        }
                    }
                    
                    div {
                        style: "background: #1e1e1e; padding: 15px; border-radius: 4px; border: 1px solid #555;",
                        div {
                            style: "color: #888; font-size: 0.9em; margin-bottom: 5px;",
                            "GPU Memory"
                        }
                        div {
                            style: "color: #2196F3; font-size: 1.5em; font-weight: bold;",
                            "{performance_stats.read().gpu_memory_mb} MB"
                        }
                    }
                    
                    div {
                        style: "background: #1e1e1e; padding: 15px; border-radius: 4px; border: 1px solid #555;",
                        div {
                            style: "color: #888; font-size: 0.9em; margin-bottom: 5px;",
                            "Surfaces"
                        }
                        div {
                            style: "color: #FF9800; font-size: 1.5em; font-weight: bold;",
                            "{performance_stats.read().active_surfaces}"
                        }
                    }
                    
                    div {
                        style: "background: #1e1e1e; padding: 15px; border-radius: 4px; border: 1px solid #555;",
                        div {
                            style: "color: #888; font-size: 0.9em; margin-bottom: 5px;",
                            "Render Time"
                        }
                        div {
                            style: "color: #9C27B0; font-size: 1.5em; font-weight: bold;",
                            "{performance_stats.read().render_time_ms:.1} ms"
                        }
                    }
                }
            }

            footer {
                style: "margin-top: 40px; padding-top: 20px; border-top: 1px solid #333; text-align: center; color: #666;",
                p {
                    "🚀 Powered by Blitz GLES + Smithay Compositor Integration"
                }
            }
        }
    }
}

#[derive(Default, Clone)]
struct PerformanceStats {
    fps: u32,
    gpu_memory_mb: u32,
    active_surfaces: u32,
    render_time_ms: f32,
}

fn initialize_integration() -> Result<BlitzSmithayIntegration> {
    use dioxus_core::prelude::consume_context;
    use mini_dxn::DxnWindowRenderer;
    
    info!("Initializing Blitz-Smithay integration");
    
    let renderer = consume_context::<DxnWindowRenderer>();
    
    if let Some(window_handle) = renderer.get_window_handle() {
        BlitzSmithayIntegration::new(window_handle.as_ref())
    } else {
        Err(anyhow::anyhow!("Window handle not available - renderer may not be resumed yet"))
    }
}

fn spawn_terminal(
    integration: Signal<Option<Arc<Mutex<BlitzSmithayIntegration>>>>,
    mut terminals: Signal<Vec<u64>>,
    mut status: Signal<String>,
    command: &str,
) {
    info!("Spawning terminal with command: {}", command);
    
    if let Some(int) = integration.read().as_ref() {
        let result = {
            let mut integration_guard = int.lock().unwrap();
            integration_guard.spawn_terminal(command)
        };
        
        match result {
            Ok(terminal_id) => {
                terminals.write().push(terminal_id);
                status.set(format!("Spawned terminal {} with command: {}", terminal_id, command));
                info!("Successfully spawned terminal {}", terminal_id);
            }
            Err(e) => {
                error!("Failed to spawn terminal: {}", e);
                status.set(format!("Error spawning terminal: {}", e));
            }
        }
    } else {
        status.set("Integration not initialized".to_string());
    }
}

fn kill_terminal(
    integration: Signal<Option<Arc<Mutex<BlitzSmithayIntegration>>>>,
    mut terminals: Signal<Vec<u64>>,
    mut status: Signal<String>,
    terminal_id: u64,
) {
    info!("Killing terminal {}", terminal_id);
    
    if let Some(int) = integration.read().as_ref() {
        let result = {
            let mut integration_guard = int.lock().unwrap();
            integration_guard.get_terminal_manager_mut().kill_terminal(terminal_id)
        };
        
        match result {
            Ok(_) => {
                terminals.write().retain(|&id| id != terminal_id);
                status.set(format!("Killed terminal {}", terminal_id));
                info!("Successfully killed terminal {}", terminal_id);
            }
            Err(e) => {
                error!("Failed to kill terminal: {}", e);
                status.set(format!("Error killing terminal: {}", e));
            }
        }
    }
}

fn kill_all_terminals(
    integration: Signal<Option<Arc<Mutex<BlitzSmithayIntegration>>>>,
    mut terminals: Signal<Vec<u64>>,
    mut status: Signal<String>,
) {
    info!("Killing all terminals");
    
    if let Some(int) = integration.read().as_ref() {
        let terminal_ids = terminals.read().clone();
        let mut killed_count = 0;
        
        for terminal_id in terminal_ids {
            let result = {
                let mut integration_guard = int.lock().unwrap();
                integration_guard.get_terminal_manager_mut().kill_terminal(terminal_id)
            };
            
            if result.is_ok() {
                killed_count += 1;
            }
        }
        
        terminals.write().clear();
        status.set(format!("Killed {} terminals", killed_count));
        info!("Successfully killed {} terminals", killed_count);
    }
}
