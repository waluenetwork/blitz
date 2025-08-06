//! 

use anyhow::Result;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

pub mod context_bridge;
pub mod event_dispatcher;
pub mod surface_manager;
pub mod terminal_manager;

pub use context_bridge::*;
pub use event_dispatcher::*;
pub use surface_manager::*;
pub use terminal_manager::*;

pub struct BlitzSmithayIntegration {
    context_bridge: Arc<Mutex<ContextBridge>>,
    event_dispatcher: EventDispatcher,
    surface_manager: SurfaceManager,
    terminal_manager: TerminalManager,
}

impl BlitzSmithayIntegration {
    pub fn new<W>(window: &W) -> Result<Self>
    where
        W: HasWindowHandle + HasDisplayHandle + ?Sized,
    {
        info!("Initializing Blitz-Smithay GLES integration");
        
        let context_bridge = Arc::new(Mutex::new(ContextBridge::new(window)?));
        let event_dispatcher = EventDispatcher::new();
        let surface_manager = SurfaceManager::new();
        let terminal_manager = TerminalManager::new();
        
        Ok(Self {
            context_bridge,
            event_dispatcher,
            surface_manager,
            terminal_manager,
        })
    }
    
    pub fn get_context_bridge(&self) -> Arc<Mutex<ContextBridge>> {
        self.context_bridge.clone()
    }
    
    pub fn get_event_dispatcher(&self) -> &EventDispatcher {
        &self.event_dispatcher
    }
    
    pub fn get_surface_manager(&self) -> &SurfaceManager {
        &self.surface_manager
    }
    
    pub fn get_terminal_manager(&self) -> &TerminalManager {
        &self.terminal_manager
    }
    
    pub fn get_terminal_manager_mut(&mut self) -> &mut TerminalManager {
        &mut self.terminal_manager
    }
    
    pub fn spawn_terminal(&mut self, command: &str) -> Result<u64> {
        debug!("Spawning terminal with command: {}", command);
        self.terminal_manager.spawn_terminal(command)
    }
    
    pub fn render_frame(&mut self) -> Result<()> {
        let mut context = self.context_bridge.lock().unwrap();
        context.make_current()?;
        
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        
        self.surface_manager.render_surfaces()?;
        
        unsafe {
            gl::Finish();
        }
        
        context.swap_buffers()?;
        
        Ok(())
    }
}
