use bevy_ecs::prelude::*;
use bevy_ecs::system::{IntoSystem, RunSystemOnce};
use bevy_ecs::query::QueryData;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum EcsContextError {
    #[error("Failed to lock ECS World: {0}")]
    LockError(String),
    #[error("System execution failed: {0}")]
    SystemError(String),
}

pub struct EcsWorldContext {
    pub(crate) world: Arc<Mutex<World>>,
}

impl EcsWorldContext {
    pub fn new(world: World) -> Self {
        Self {
            world: Arc::new(Mutex::new(world)),
        }
    }

    pub fn run_system<S, Out, Marker>(&self, system: S) -> Result<Out, EcsContextError>
    where
        S: IntoSystem<(), Out, Marker> + 'static,
        Out: 'static,
    {
        let mut world = self.world.lock()
            .map_err(|e| EcsContextError::LockError(e.to_string()))?;
        
        Ok(world.run_system_once(system))
    }

    pub fn get_resource<R: bevy_ecs::system::Resource + Clone>(&self) -> Result<Option<R>, EcsContextError> {
        let world = self.world.lock()
            .map_err(|e| EcsContextError::LockError(e.to_string()))?;
        
        Ok(world.get_resource::<R>().cloned())
    }

    pub fn query_count<Q: QueryData>(&self) -> Result<usize, EcsContextError> {
        let mut world = self.world.lock()
            .map_err(|e| EcsContextError::LockError(e.to_string()))?;
        
        let mut query = world.query::<Q>();
        Ok(query.iter(&world).count())
    }
}

impl Clone for EcsWorldContext {
    fn clone(&self) -> Self {
        Self {
            world: Arc::clone(&self.world),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;

    #[derive(bevy_ecs::system::Resource, Clone, PartialEq, Debug)]
    struct TestResource(String);

    #[derive(Component, Clone, PartialEq, Debug)]
    struct TestComponent(i32);

    fn test_system(query: Query<&TestComponent>) -> usize {
        query.iter().count()
    }

    #[test]
    fn test_ecs_context_creation() {
        let world = World::new();
        let context = EcsWorldContext::new(world);
        assert!(context.world.lock().is_ok());
    }

    #[test]
    fn test_resource_access() {
        let mut world = World::new();
        world.insert_resource(TestResource("test".to_string()));
        
        let context = EcsWorldContext::new(world);
        let resource = context.get_resource::<TestResource>().unwrap();
        
        assert_eq!(resource, Some(TestResource("test".to_string())));
    }

    #[test]
    fn test_system_execution() {
        let mut world = World::new();
        world.spawn(TestComponent(1));
        world.spawn(TestComponent(2));
        
        let context = EcsWorldContext::new(world);
        let result = context.run_system(test_system).unwrap();
        
        assert_eq!(result, 2);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;
        
        let mut world = World::new();
        world.insert_resource(TestResource("shared".to_string()));
        
        let context = EcsWorldContext::new(world);
        let context_clone = context.clone();
        
        let handle = thread::spawn(move || {
            context_clone.get_resource::<TestResource>().unwrap()
        });
        
        let result = handle.join().unwrap();
        assert_eq!(result, Some(TestResource("shared".to_string())));
    }

    #[test]
    fn test_query_count() {
        let mut world = World::new();
        world.spawn(TestComponent(1));
        world.spawn(TestComponent(2));
        world.spawn(TestComponent(3));
        
        let context = EcsWorldContext::new(world);
        let count = context.query_count::<&TestComponent>().unwrap();
        
        assert_eq!(count, 3);
    }
}
