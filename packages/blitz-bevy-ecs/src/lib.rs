//! 

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod ecs_context;
pub mod hooks;
pub mod document;
pub mod change_detection;

pub mod prelude {
    
    pub use crate::ecs_context::{EcsWorldContext, EcsContextError};
    pub use crate::hooks::*;
    pub use crate::document::EcsDioxusDocument;
    pub use crate::change_detection::EcsChangeDetector;
    
    pub use bevy_ecs::prelude::*;
    pub use dioxus::prelude::*;
    pub use mini_dxn::DioxusDocument;
    pub use blitz_dom::{BaseDocument, Document};
}

pub use crate::ecs_context::{EcsWorldContext, EcsContextError};
pub use crate::document::EcsDioxusDocument;
pub use crate::change_detection::EcsChangeDetector;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
