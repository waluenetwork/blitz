use crate::ecs_context::{EcsWorldContext, EcsContextError};
use bevy_ecs::system::IntoSystem;
use bevy_ecs::query::QueryData;
use dioxus::prelude::*;

pub fn use_ecs_system<S, Out, Marker>(
    system: S
) -> ReadOnlySignal<Result<Out, EcsContextError>>
where
    S: IntoSystem<(), Out, Marker> + 'static + Clone,
    Out: 'static + Clone + PartialEq,
{
    let ecs_context = consume_context::<EcsWorldContext>();
    
    use_memo(move || {
        ecs_context.run_system(system.clone())
    }).into()
}

pub fn use_ecs_resource<R: bevy_ecs::system::Resource + Clone + PartialEq + 'static>() -> ReadOnlySignal<Option<R>> {
    let ecs_context = consume_context::<EcsWorldContext>();
    
    use_memo(move || {
        ecs_context.get_resource::<R>().unwrap_or(None)
    }).into()
}

pub fn use_ecs_query_count<Q>() -> ReadOnlySignal<usize>
where
    Q: QueryData + 'static,
{
    let ecs_context = consume_context::<EcsWorldContext>();
    
    use_memo(move || {
        ecs_context.query_count::<Q>().unwrap_or(0)
    }).into()
}

pub fn use_ecs_system_simple<S, Out, Marker>(
    system: S
) -> ReadOnlySignal<Result<Out, EcsContextError>>
where
    S: IntoSystem<(), Out, Marker> + 'static + Clone,
    Out: 'static + Clone + PartialEq,
{
    use_ecs_system(system)
}

pub fn use_ecs_has_resource<R: bevy_ecs::system::Resource + Clone + 'static>() -> ReadOnlySignal<bool> {
    let ecs_context = consume_context::<EcsWorldContext>();
    
    use_memo(move || {
        ecs_context.get_resource::<R>().map(|r| r.is_some()).unwrap_or(false)
    }).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;

    #[derive(bevy_ecs::system::Resource, Clone)]
    struct TestResource(pub String);

    #[derive(Component)]
    struct TestComponent(pub i32);

    fn test_system() -> String {
        "system_result".to_string()
    }

    #[test]
    fn test_hooks_compile() {
        assert!(true);
    }
}
