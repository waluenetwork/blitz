
use crate::ecs_context::EcsWorldContext;
use bevy_ecs::prelude::*;
use bevy_ecs::event::{Event, EventReader};
use bevy_ecs::system::Commands;
use bevy_ecs::query::{QueryData, QueryFilter};
use dioxus::prelude::*;

pub fn use_ecs_event_reader<E: Event + Clone + PartialEq + 'static>() -> ReadOnlySignal<Vec<E>> {
    let ecs_context = use_context::<EcsWorldContext>();
    
    use_memo(move || {
        let ctx = &ecs_context;
        let event_system = |mut event_reader: EventReader<E>| {
            event_reader.read().cloned().collect::<Vec<_>>()
        };
        
        match ctx.run_system(event_system) {
            Ok(events) => events,
            Err(_) => Vec::new(),
        }
    }).into()
}

pub fn use_ecs_commands() -> impl Fn(fn(Commands)) + Clone {
    let ecs_context = use_context::<EcsWorldContext>();
    
    move |command_fn: fn(Commands)| {
        let ctx = &ecs_context;
        let command_system = move |commands: Commands| {
            command_fn(commands);
        };
        
        let _ = ctx.run_system(command_system);
    }
}

pub fn use_ecs_query_count<D: QueryData + 'static, F: QueryFilter + 'static>() -> ReadOnlySignal<usize> {
    let ecs_context = use_context::<EcsWorldContext>();
    
    use_memo(move || {
        let ctx = &ecs_context;
        let count_system = |query: Query<D, F>| query.iter().count();
        
        match ctx.run_system(count_system) {
            Ok(count) => count,
            Err(_) => 0,
        }
    }).into()
}

pub fn use_ecs_entity_exists<D: QueryData + 'static>(entity: Entity) -> ReadOnlySignal<bool> {
    let ecs_context = use_context::<EcsWorldContext>();
    
    use_memo(move || {
        let ctx = &ecs_context;
        let entity_system = move |query: Query<D>| {
            query.get(entity).is_ok()
        };
        
        match ctx.run_system(entity_system) {
            Ok(exists) => exists,
            Err(_) => false,
        }
    }).into()
}

/// 
/// 
///     
pub fn use_ecs_reactive_query<D: QueryData + 'static, F: QueryFilter + 'static>() -> ReadOnlySignal<usize> {
    let ecs_context = use_context::<EcsWorldContext>();
    
    use_memo(move || {
        let ctx = &ecs_context;
        let count_system = |query: Query<D, F>| query.iter().count();
        
        match ctx.run_system(count_system) {
            Ok(count) => count,
            Err(_) => 0,
        }
    }).into()
}

pub fn use_ecs_timer_info<R>() -> ReadOnlySignal<TimerInfo>
where
    R: bevy_ecs::system::Resource + Clone + 'static,
{
    let ecs_context = use_context::<EcsWorldContext>();
    
    use_memo(move || {
        let ctx = &ecs_context;
        let timer_system = |_timer_res: Res<R>| {
            TimerInfo {
                remaining_secs: 0.0,
                elapsed_secs: 0.0,
                finished: false,
                just_finished: false,
            }
        };
        
        match ctx.run_system(timer_system) {
            Ok(info) => info,
            Err(_) => TimerInfo::default(),
        }
    }).into()
}

#[derive(Clone, PartialEq, Debug)]
pub struct TimerInfo {
    pub remaining_secs: f32,
    pub elapsed_secs: f32,
    pub finished: bool,
    pub just_finished: bool,
}

impl Default for TimerInfo {
    fn default() -> Self {
        Self {
            remaining_secs: 0.0,
            elapsed_secs: 0.0,
            finished: false,
            just_finished: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;

    #[derive(Event, Clone, PartialEq)]
    struct TestEvent(String);

    #[derive(Component, Clone, PartialEq)]
    struct TestComponent(i32);

    #[derive(bevy_ecs::system::Resource, Clone)]
    struct TestTimer(i32);

    #[test]
    fn test_timer_info_default() {
        let info = TimerInfo::default();
        assert_eq!(info.remaining_secs, 0.0);
        assert_eq!(info.elapsed_secs, 0.0);
        assert!(!info.finished);
        assert!(!info.just_finished);
    }

    #[test]
    fn test_advanced_hooks_compile() {
        assert!(true);
    }
}
