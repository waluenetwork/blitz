//!

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod ecs_context;
pub mod hooks;
pub mod document;
pub mod change_detection;
pub mod components;
pub mod advanced_hooks;
pub mod static_generation;
pub mod routing;
pub mod integration_tests;
pub mod performance;

pub mod prelude {
    
    pub use crate::ecs_context::{EcsWorldContext, EcsContextError};
    pub use crate::hooks::{use_ecs_resource, use_ecs_system, use_ecs_query_count, use_ecs_system_simple, use_ecs_has_resource};
    pub use crate::document::EcsDioxusDocument;
    pub use crate::change_detection::EcsChangeDetector;
    pub use crate::components::*;
    pub use crate::advanced_hooks::{use_ecs_event_reader, use_ecs_commands, use_ecs_entity_exists, use_ecs_reactive_query, use_ecs_timer_info, TimerInfo};
    pub use crate::static_generation::{StaticSiteGenerator, RouteConfig, AssetProcessor};
    pub use crate::routing::{Router, Route, SiteMap};
    pub use crate::integration_tests::{IntegrationTestSuite, IntegrationTestConfig, TestMetrics, BenchmarkSuite};
    pub use crate::performance::{PerformanceProfiler, PerformanceAnalyzer, PerformanceReport, ProfiledEcsContext};
    
    pub use bevy_ecs::prelude::*;
    pub use dioxus::prelude::*;
    pub use mini_dxn::DioxusDocument;
    pub use blitz_dom::{BaseDocument, Document};
}

pub use crate::ecs_context::{EcsWorldContext, EcsContextError};
pub use crate::document::EcsDioxusDocument;
pub use crate::change_detection::EcsChangeDetector;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
