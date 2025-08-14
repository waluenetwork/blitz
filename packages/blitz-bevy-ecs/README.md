# blitz-bevy-ecs

Bevy ECS integration for Blitz + Dioxus applications, enabling reactive UI components that can access and respond to ECS world state.

## Features

- **Seamless ECS Integration**: Access Bevy ECS World from Dioxus components
- **Reactive Hooks**: Use ECS resources, components, and systems in Dioxus hooks
- **Change Detection**: Efficient change detection using Bevy's tick system
- **Real Blitz Integration**: Built on top of real Blitz dependencies, no mocks
- **Thread Safety**: Safe concurrent access to ECS World via Arc<Mutex<World>>

## Usage

```rust
use blitz_bevy_ecs::prelude::*;

#[derive(Resource, Clone, PartialEq)]
struct Counter(i32);

#[component]
fn App() -> Element {
    let counter = use_ecs_resource::<Counter>();
    
    rsx! {
        div {
            "Counter: {counter.read().as_ref().map(|c| c.0).unwrap_or(0)}"
        }
    }
}

#[tokio::main]
async fn main() {
    let mut world = World::new();
    world.insert_resource(Counter(42));
    
    let vdom = VirtualDom::new(App);
    let mut doc = EcsDioxusDocument::new(vdom, world, None);
    
    doc.track_resource::<Counter>();
}
```

## Hooks

- `use_ecs_resource<R>()` - Access ECS resources reactively
- `use_ecs_system<S>()` - Run ECS systems and get results
- `use_ecs_query_count<Q>()` - Count entities matching a query
- `use_ecs_has_resource<R>()` - Check if a resource exists

## Architecture

This crate provides:

1. **EcsWorldContext** - Thread-safe wrapper around Bevy World
2. **EcsDioxusDocument** - Document implementation that integrates ECS with Dioxus
3. **EcsChangeDetector** - Efficient change detection for ECS state
4. **Dioxus Hooks** - Reactive hooks for accessing ECS data

## Integration with Blitz

The `EcsDioxusDocument` wraps `mini_dxn::DioxusDocument` and implements `blitz_dom::Document`, providing seamless integration with the Blitz rendering pipeline while adding ECS capabilities.

## License

MIT OR Apache-2.0
