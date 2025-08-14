use bevy_ecs::prelude::*;
use bevy_ecs::component::ComponentId;
use std::collections::HashSet;

pub struct EcsChangeDetector {
    last_change_tick: bevy_ecs::component::Tick,
    tracked_components: HashSet<ComponentId>,
    tracked_resources: HashSet<ComponentId>,
}

impl EcsChangeDetector {
    pub fn new() -> Self {
        Self {
            last_change_tick: bevy_ecs::component::Tick::new(0),
            tracked_components: HashSet::new(),
            tracked_resources: HashSet::new(),
        }
    }

    pub fn track_component<T: Component>(&mut self, world: &World) {
        if let Some(component_id) = world.components().get_id(std::any::TypeId::of::<T>()) {
            self.tracked_components.insert(component_id);
        }
    }

    pub fn track_resource<R: bevy_ecs::system::Resource>(&mut self, world: &World) {
        if let Some(component_id) = world.components().get_resource_id(std::any::TypeId::of::<R>()) {
            self.tracked_resources.insert(component_id);
        }
    }

    pub fn check_changes(&mut self, world: &mut World) -> bool {
        let current_tick = world.change_tick();
        
        if self.last_change_tick.get() == 0 {
            self.last_change_tick = current_tick;
            return false;
        }
        
        if current_tick == self.last_change_tick {
            return false;
        }
        
        let has_changes = current_tick != self.last_change_tick;
        
        self.last_change_tick = current_tick;
        has_changes
    }

    pub fn reset(&mut self, world: &mut World) {
        self.last_change_tick = world.change_tick();
    }
}

impl Default for EcsChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(bevy_ecs::component::Component)]
    struct TestComponent(i32);

    #[derive(bevy_ecs::system::Resource)]
    struct TestResource(String);

    #[test]
    fn test_change_detection() {
        let mut world = World::new();
        let mut detector = EcsChangeDetector::new();
        
        detector.track_component::<TestComponent>(&world);
        detector.track_resource::<TestResource>(&world);
        
        assert!(!detector.check_changes(&mut world));
    }
}
