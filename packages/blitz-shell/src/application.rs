use crate::event::BlitzShellEvent;

use anyrender::WindowRenderer;
use std::collections::HashMap;
#[cfg(feature = "tracing")]
use tracing::debug;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::{View, WindowConfig};

pub struct BlitzApplication<Rend: WindowRenderer> {
    pub windows: HashMap<WindowId, View<Rend>>,
    pub pending_windows: Vec<WindowConfig<Rend>>,
    pub proxy: EventLoopProxy<BlitzShellEvent>,
}

impl<Rend: WindowRenderer> BlitzApplication<Rend> {
    pub fn new(proxy: EventLoopProxy<BlitzShellEvent>) -> Self {
        BlitzApplication {
            windows: HashMap::new(),
            pending_windows: Vec::new(),
            proxy,
        }
    }

    pub fn add_window(&mut self, window_config: WindowConfig<Rend>) {
        self.pending_windows.push(window_config);
    }

    fn window_mut_by_doc_id(&mut self, doc_id: usize) -> Option<&mut View<Rend>> {
        self.windows.values_mut().find(|w| w.doc.id() == doc_id)
    }
}

impl<Rend: WindowRenderer> ApplicationHandler<BlitzShellEvent> for BlitzApplication<Rend> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(feature = "tracing")]
        debug!("BlitzApplication::resumed() called with {} existing windows and {} pending windows", 
               self.windows.len(), self.pending_windows.len());
        
        // Resume existing windows
        for (window_id, view) in self.windows.iter_mut() {
            #[cfg(feature = "tracing")]
            debug!("Resuming existing window {:?}", window_id);
            view.resume();
        }

        // Initialise pending windows
        for (index, window_config) in self.pending_windows.drain(..).enumerate() {
            #[cfg(feature = "tracing")]
            debug!("Initializing pending window {}", index);
            let mut view = View::init(window_config, event_loop, &self.proxy);
            #[cfg(feature = "tracing")]
            debug!("View initialized, calling resume...");
            view.resume();
            
            let is_active = view.renderer.is_active();
            #[cfg(feature = "tracing")]
            debug!("Renderer is_active: {}", is_active);
            
            if !is_active {
                #[cfg(feature = "tracing")]
                debug!("WARNING: Renderer is not active, window will not be added to application");
                continue;
            }
            
            let window_id = view.window_id();
            #[cfg(feature = "tracing")]
            debug!("Adding window {:?} to application", window_id);
            self.windows.insert(window_id, view);
        }
        
        #[cfg(feature = "tracing")]
        debug!("BlitzApplication::resumed() completed with {} total windows", self.windows.len());
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        for (_, view) in self.windows.iter_mut() {
            view.suspend();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Exit the app when window close is requested.
        if matches!(event, WindowEvent::CloseRequested) {
            // Drop window before exiting event loop
            // See https://github.com/rust-windowing/winit/issues/4135
            let window = self.windows.remove(&window_id);
            drop(window);
            if self.windows.is_empty() {
                event_loop.exit();
            }
            return;
        }

        if let Some(window) = self.windows.get_mut(&window_id) {
            window.handle_winit_event(event);
        }

        let _ = self.proxy.send_event(BlitzShellEvent::Poll { window_id });
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: BlitzShellEvent) {
        match event {
            BlitzShellEvent::Poll { window_id } => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.poll();
                };
            }
            BlitzShellEvent::ResourceLoad { doc_id, data } => {
                // TODO: Handle multiple documents per window
                if let Some(window) = self.window_mut_by_doc_id(doc_id) {
                    window.doc.as_mut().load_resource(data);
                    window.request_redraw();
                }
            }

            #[cfg(feature = "accessibility")]
            BlitzShellEvent::Accessibility { window_id, data } => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    match &*data {
                        accesskit_winit::WindowEvent::InitialTreeRequested => {
                            window.build_accessibility_tree();
                        }
                        accesskit_winit::WindowEvent::AccessibilityDeactivated => {
                            // TODO
                        }
                        accesskit_winit::WindowEvent::ActionRequested(_req) => {
                            // TODO
                        }
                    }
                }
            }

            BlitzShellEvent::Embedder(_) => {
                // Do nothing. Should be handled by embedders (if required).
            }
            BlitzShellEvent::Navigate(_opts) => {
                // Do nothing. Should be handled by embedders (if required).
            }
            BlitzShellEvent::NavigationLoad { .. } => {
                // Do nothing. Should be handled by embedders (if required).
            }
        }
    }
}
