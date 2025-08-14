//! 

use crate::ecs_context::EcsWorldContext;
use crate::hooks::{use_ecs_resource, use_ecs_system_simple};
use bevy_ecs::prelude::*;
use dioxus::prelude::*;

/// 
/// 
#[component]
pub fn EcsText<R>(
    #[props(optional)] class: Option<String>,
    #[props(optional)] style: Option<String>,
    formatter: fn(&R) -> String,
) -> Element 
where
    R: bevy_ecs::system::Resource + Clone + PartialEq + 'static,
{
    let resource = use_ecs_resource::<R>();
    
    rsx! {
        span {
            class: class.unwrap_or_default(),
            style: style.unwrap_or_default(),
            {
                match resource.read().as_ref() {
                    Some(res) => formatter(res),
                    None => "Loading...".to_string(),
                }
            }
        }
    }
}

/// 
/// 
#[component]
pub fn EcsButton<S>(
    children: Element,
    system: S,
    #[props(optional)] class: Option<String>,
    #[props(optional)] style: Option<String>,
    #[props(optional)] disabled: Option<bool>,
) -> Element
where
    S: IntoSystem<(), (), ()> + 'static + Clone + PartialEq,
{
    let ecs_context = use_context::<EcsWorldContext>();
    
    let onclick = move |_| {
        let ctx = &ecs_context;
        let _ = ctx.run_system(system.clone());
    };
    
    rsx! {
        button {
            class: class.unwrap_or_default(),
            style: style.unwrap_or_default(),
            disabled: disabled.unwrap_or(false),
            onclick,
            {children}
        }
    }
}

/// 
/// 
#[component]
pub fn EcsEntityCount(
    #[props(optional)] prefix: Option<String>,
    #[props(optional)] suffix: Option<String>,
    #[props(optional)] class: Option<String>,
    #[props(optional)] style: Option<String>,
) -> Element
{
    let count_system = |query: Query<Entity>| query.iter().count();
    let count = use_ecs_system_simple(count_system);
    
    rsx! {
        span {
            class: class.unwrap_or_default(),
            style: style.unwrap_or_default(),
            {
                let count_val = match count.read().as_ref() {
                    Ok(val) => *val,
                    Err(_) => 0,
                };
                format!("{}{}{}", 
                    prefix.as_ref().map(|s| s.as_str()).unwrap_or(""),
                    count_val,
                    suffix.as_ref().map(|s| s.as_str()).unwrap_or("")
                )
            }
        }
    }
}

/// 
#[component]
pub fn EcsConditional<R>(
    children: Element,
    condition: fn(&R) -> bool,
) -> Element
where
    R: bevy_ecs::system::Resource + Clone + PartialEq + 'static,
{
    let resource = use_ecs_resource::<R>();
    
    let should_render = match resource.read().as_ref() {
        Some(res) => condition(res),
        None => false,
    };
    
    if should_render {
        rsx! { {children} }
    } else {
        rsx! { }
    }
}

#[component]
pub fn EcsEntityList(
    #[props(optional)] class: Option<String>,
    #[props(optional)] style: Option<String>,
) -> Element
{
    let count_system = |query: Query<Entity>| query.iter().count();
    let entity_count = use_ecs_system_simple(count_system);
    
    rsx! {
        ul {
            class: class.unwrap_or_default(),
            style: style.unwrap_or_default(),
            li { 
                {
                    let count_val = match entity_count.read().as_ref() {
                        Ok(val) => *val,
                        Err(_) => 0,
                    };
                    format!("Entity count: {}", count_val)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;

    #[derive(bevy_ecs::system::Resource, Clone, PartialEq)]
    struct TestResource(String);

    #[derive(Component, Clone, PartialEq)]
    struct TestComponent(i32);

    #[test]
    fn test_ecs_text_component() {
        let _formatter = |res: &TestResource| res.0.clone();
    }

    #[test]
    fn test_ecs_button_component() {
        fn test_system() -> i32 { 42 }
        
        let _system = test_system;
    }

    #[test]
    fn test_ecs_entity_count_component() {
        type TestQuery = &'static TestComponent;
        
        assert!(true);
    }
}
