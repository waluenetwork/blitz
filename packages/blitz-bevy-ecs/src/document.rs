use crate::ecs_context::EcsWorldContext;
use crate::change_detection::EcsChangeDetector;
use bevy_ecs::prelude::*;
use bevy_ecs::component::Component as BevyComponent;
use dioxus::prelude::*;
use dioxus_core::VirtualDom;
use std::sync::Arc;
use std::any::Any;
use std::task::Context as TaskContext;
use std::ops::{Deref, DerefMut};

use blitz_dom::{BaseDocument, Document};
use blitz_traits::net::NetProvider;
use blitz_traits::events::UiEvent;
use mini_dxn::DioxusDocument;
use blitz_dom::net::Resource;

pub struct EcsDioxusDocument {
    inner: DioxusDocument,
    ecs_context: EcsWorldContext,
    change_detector: EcsChangeDetector,
}

impl EcsDioxusDocument {
    pub fn new(
        vdom: VirtualDom, 
        world: World, 
        net_provider: Option<Arc<dyn NetProvider<Resource>>>
    ) -> Self {
        let ecs_context = EcsWorldContext::new(world);
        
        vdom.provide_root_context(ecs_context.clone());
        
        let inner = DioxusDocument::new(vdom, net_provider);
        let change_detector = EcsChangeDetector::new();
        
        Self {
            inner,
            ecs_context,
            change_detector,
        }
    }

    pub fn update_from_ecs(&mut self) -> bool {
        let mut world_guard = match self.ecs_context.world.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        
        self.change_detector.check_changes(&mut *world_guard)
    }

    pub fn ecs_context(&self) -> &EcsWorldContext {
        &self.ecs_context
    }

    pub fn track_component<T: BevyComponent>(&mut self) {
        if let Ok(world) = self.ecs_context.world.lock() {
            self.change_detector.track_component::<T>(&*world);
        }
    }

    pub fn track_resource<R: bevy_ecs::system::Resource>(&mut self) {
        if let Ok(world) = self.ecs_context.world.lock() {
            self.change_detector.track_resource::<R>(&*world);
        }
    }
}

impl Document for EcsDioxusDocument {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn handle_ui_event(&mut self, event: UiEvent) {
        self.inner.handle_ui_event(event);
    }

    fn poll(&mut self, cx: Option<TaskContext>) -> bool {
        let ecs_changed = self.update_from_ecs();
        
        let inner_changed = self.inner.poll(cx);
        
        ecs_changed || inner_changed
    }
}

impl Deref for EcsDioxusDocument {
    type Target = BaseDocument;
    
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl DerefMut for EcsDioxusDocument {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;

    #[derive(bevy_ecs::system::Resource, Clone)]
    struct TestResource(String);

    #[derive(bevy_ecs::component::Component)]
    struct TestComponent(i32);

    #[test]
    fn test_ecs_dioxus_document_creation() {
        let world = World::new();
        let vdom = VirtualDom::new(|| rsx! { div { "test" } });
        let doc = EcsDioxusDocument::new(vdom, world, None);
        assert!(doc.ecs_context().world.lock().is_ok());
    }

    #[test]
    fn test_change_tracking() {
        let mut world = World::new();
        world.insert_resource(TestResource("test".to_string()));
        
        let vdom = VirtualDom::new(|| rsx! { div { "test" } });
        let mut doc = EcsDioxusDocument::new(vdom, world, None);
        
        doc.track_resource::<TestResource>();
        assert!(!doc.update_from_ecs());
    }

    #[test]
    fn test_real_blitz_integration() {
        let world = World::new();
        let vdom = VirtualDom::new(|| rsx! { div { "real blitz test" } });
        let mut doc = EcsDioxusDocument::new(vdom, world, None);
        
        let result = doc.poll(None);
        assert!(!result);
        
        assert!(doc.ecs_context().world.lock().is_ok());
    }
}
