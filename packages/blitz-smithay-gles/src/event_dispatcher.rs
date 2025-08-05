
use anyhow::Result;
use rustc_hash::FxHashMap;
use std::any::Any;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub enum EventTarget {
    Dom,
    Wayland,
    Terminal(u64),
}

#[derive(Debug)]
pub struct Event {
    pub target: EventTarget,
    pub event_type: String,
    pub data: Box<dyn Any + Send + Sync>,
}

pub struct EventDispatcher {
    event_handlers: FxHashMap<String, Vec<Box<dyn Fn(&Event) -> Result<bool> + Send + Sync>>>,
    focus_target: Option<EventTarget>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        debug!("Creating event dispatcher");
        
        Self {
            event_handlers: FxHashMap::default(),
            focus_target: None,
        }
    }
    
    pub fn register_handler<F>(&mut self, event_type: &str, handler: F)
    where
        F: Fn(&Event) -> Result<bool> + Send + Sync + 'static,
    {
        self.event_handlers
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(handler));
        
        debug!("Registered handler for event type: {}", event_type);
    }
    
    pub fn dispatch_event(&self, event: Event) -> Result<bool> {
        debug!("Dispatching event: {:?}", event.event_type);
        
        if let Some(handlers) = self.event_handlers.get(&event.event_type) {
            for handler in handlers {
                if handler(&event)? {
                    debug!("Event handled by handler");
                    return Ok(true);
                }
            }
        }
        
        warn!("No handler found for event type: {}", event.event_type);
        Ok(false)
    }
    
    pub fn set_focus(&mut self, target: EventTarget) {
        debug!("Setting focus to: {:?}", target);
        self.focus_target = Some(target);
    }
    
    pub fn get_focus(&self) -> Option<&EventTarget> {
        self.focus_target.as_ref()
    }
    
    pub fn route_keyboard_event(&self, _key_event: &dyn Any) -> Result<EventTarget> {
        if let Some(target) = &self.focus_target {
            Ok(target.clone())
        } else {
            Ok(EventTarget::Dom)
        }
    }
    
    pub fn route_mouse_event(&self, x: f32, y: f32) -> Result<EventTarget> {
        if x > 100.0 && y > 100.0 {
            Ok(EventTarget::Wayland)
        } else {
            Ok(EventTarget::Dom)
        }
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
