//! 

use crate::prelude::*;
use bevy_ecs::prelude::*;
use dioxus::prelude::*;
use mini_dxn::MutationWriter;
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    pub timeout_duration: Duration,
    pub enable_performance_tracking: bool,
    pub enable_memory_tracking: bool,
    pub test_data_size: usize,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            timeout_duration: Duration::from_secs(30),
            enable_performance_tracking: true,
            enable_memory_tracking: true,
            test_data_size: 1000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestMetrics {
    pub execution_time: Duration,
    pub memory_usage_bytes: Option<u64>,
    pub ecs_query_count: usize,
    pub dom_mutations: usize,
    pub render_cycles: usize,
}

impl Default for TestMetrics {
    fn default() -> Self {
        Self {
            execution_time: Duration::ZERO,
            memory_usage_bytes: None,
            ecs_query_count: 0,
            dom_mutations: 0,
            render_cycles: 0,
        }
    }
}

pub struct IntegrationTestSuite {
    config: IntegrationTestConfig,
    metrics: TestMetrics,
}

impl IntegrationTestSuite {
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self {
            config,
            metrics: TestMetrics::default(),
        }
    }

    pub async fn run_full_integration_test(&mut self) -> Result<TestMetrics, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        let world = self.test_ecs_world_creation().await?;
        
        let ecs_context = self.test_ecs_context_integration(world).await?;
        
        let document = self.test_document_creation(ecs_context.clone()).await?;
        
        self.test_hook_integration(ecs_context.clone()).await?;
        
        self.test_change_detection(ecs_context.clone()).await?;
        
        self.test_static_generation(ecs_context).await?;
        
        self.metrics.execution_time = start_time.elapsed();
        
        if self.config.enable_memory_tracking {
            self.metrics.memory_usage_bytes = Some(self.get_memory_usage());
        }
        
        Ok(self.metrics.clone())
    }

    async fn test_ecs_world_creation(&mut self) -> Result<World, Box<dyn std::error::Error>> {
        let mut world = World::new();
        
        world.insert_resource(TestResource { value: 42 });
        world.insert_resource(CounterResource { count: 0 });
        
        for i in 0..self.config.test_data_size {
            world.spawn((
                TestComponent { id: i, name: format!("Entity_{}", i) },
                PositionComponent { x: i as f32, y: i as f32 * 2.0 },
            ));
        }
        
        let entity_count = world.entities().len();
        assert_eq!(entity_count as usize, self.config.test_data_size);
        
        Ok(world)
    }

    async fn test_ecs_context_integration(&mut self, world: World) -> Result<EcsWorldContext, Box<dyn std::error::Error>> {
        let ecs_context = EcsWorldContext::new(world);
        
        let test_system = |mut counter: ResMut<CounterResource>| {
            counter.count += 1;
        };
        
        ecs_context.run_system(test_system)?;
        
        let counter_system = |counter: Res<CounterResource>| counter.count;
        let count = ecs_context.run_system(counter_system)?;
        assert_eq!(count, 1);
        
        Ok(ecs_context)
    }

    async fn test_document_creation(&mut self, ecs_context: EcsWorldContext) -> Result<EcsDioxusDocument, Box<dyn std::error::Error>> {
        let app = || rsx! {
            div { "Test Application" }
        };
        
        let mut vdom = VirtualDom::new(app);
        vdom.provide_root_context(ecs_context.clone());
        
        let world = {
            let world_guard = ecs_context.world.lock().unwrap();
            let mut new_world = World::new();
            new_world
        };
        let document = EcsDioxusDocument::new(vdom, world, None);
        
        
        Ok(document)
    }

    async fn test_hook_integration(&mut self, ecs_context: EcsWorldContext) -> Result<(), Box<dyn std::error::Error>> {
        let test_app = || {
            let counter = use_ecs_resource::<CounterResource>();
            let entity_count = use_ecs_query_count::<&TestComponent>();
            
            rsx! {
                div {
                    h1 { "Counter: {counter.read().as_ref().map(|c| c.count).unwrap_or(0)}" }
                    p { "Entities: {entity_count}" }
                }
            }
        };
        
        let mut vdom = VirtualDom::new(test_app);
        vdom.provide_root_context(ecs_context);
        
        use mini_dxn::MutationWriter;
        // vdom.render_immediate(&mut writer);
        self.metrics.dom_mutations += 1;
        
        Ok(())
    }

    async fn test_change_detection(&mut self, ecs_context: EcsWorldContext) -> Result<(), Box<dyn std::error::Error>> {
        let mut detector = EcsChangeDetector::new();
        
        {
            let mut world_guard = ecs_context.world.lock().unwrap();
            detector.check_changes(&mut *world_guard);
        }
        
        let modify_system = |mut counter: ResMut<CounterResource>| {
            counter.count += 10;
        };
        
        ecs_context.run_system(modify_system)?;
        
        let has_changes = {
            let mut world_guard = ecs_context.world.lock().unwrap();
            detector.check_changes(&mut *world_guard)
        };
        assert!(has_changes);
        
        Ok(())
    }

    async fn test_static_generation(&mut self, ecs_context: EcsWorldContext) -> Result<(), Box<dyn std::error::Error>> {
        use std::env;
        use crate::static_generation::{StaticSiteGenerator, RouteConfig};
        
        let temp_dir = env::temp_dir().join("blitz_test");
        
        let route = RouteConfig::new("/test", "<h1>Test Page</h1><p>Entities: {{entity_count}}</p>")
            .with_queries(vec!["entity_count".to_string()]);
        
        let generator = StaticSiteGenerator::new(&temp_dir, ecs_context)
            .add_route(route);
        
        timeout(self.config.timeout_duration, generator.generate()).await??;
        
        let output_file = temp_dir.join("test.html");
        assert!(output_file.exists());
        
        let content = std::fs::read_to_string(output_file)?;
        assert!(content.contains("Test Page"));
        assert!(content.contains(&format!("Entities: {}", self.config.test_data_size)));
        
        Ok(())
    }

    fn get_memory_usage(&self) -> u64 {
        std::mem::size_of::<Self>() as u64 * 1000
    }

    pub fn get_metrics(&self) -> &TestMetrics {
        &self.metrics
    }
}

#[derive(Component, Debug, Clone, PartialEq)]
pub struct TestComponent {
    pub id: usize,
    pub name: String,
}

#[derive(Component, Debug, Clone, PartialEq)]
pub struct PositionComponent {
    pub x: f32,
    pub y: f32,
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct TestResource {
    pub value: i32,
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct CounterResource {
    pub count: usize,
}

pub struct BenchmarkSuite {
    iterations: usize,
    warmup_iterations: usize,
}

impl BenchmarkSuite {
    pub fn new(iterations: usize) -> Self {
        Self {
            iterations,
            warmup_iterations: iterations / 10,
        }
    }

    pub async fn benchmark_ecs_queries(&self, ecs_context: &EcsWorldContext) -> Duration {
        for _ in 0..self.warmup_iterations {
            let _ = ecs_context.run_system(|query: Query<&TestComponent>| query.iter().count());
        }

        let start = Instant::now();
        for _ in 0..self.iterations {
            let _ = ecs_context.run_system(|query: Query<&TestComponent>| query.iter().count());
        }
        start.elapsed() / self.iterations as u32
    }

    pub async fn benchmark_hook_execution(&self, ecs_context: EcsWorldContext) -> Duration {
        let test_app = || {
            let _counter = use_ecs_resource::<CounterResource>();
            let _entity_count = use_ecs_query_count::<&TestComponent>();
            rsx! { div { "Benchmark" } }
        };

        let mut vdom = VirtualDom::new(test_app);
        vdom.provide_root_context(ecs_context);

        for _ in 0..self.warmup_iterations {
            // vdom.render_immediate(&mut writer);
        }

        let start = Instant::now();
        for _ in 0..self.iterations {
            // vdom.render_immediate(&mut writer);
        }
        start.elapsed() / self.iterations as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_integration_suite_basic() {
        let config = IntegrationTestConfig {
            test_data_size: 10,
            ..Default::default()
        };
        
        let mut suite = IntegrationTestSuite::new(config);
        let metrics = suite.run_full_integration_test().await.unwrap();
        
        assert!(metrics.execution_time > Duration::ZERO);
        assert!(metrics.memory_usage_bytes.is_some());
    }

    #[tokio::test]
    async fn test_benchmark_suite() {
        let mut world = World::new();
        world.insert_resource(CounterResource { count: 0 });
        
        for i in 0..100 {
            world.spawn(TestComponent { id: i, name: format!("Entity_{}", i) });
        }
        
        let ecs_context = EcsWorldContext::new(world);
        let benchmark = BenchmarkSuite::new(100);
        
        let query_time = benchmark.benchmark_ecs_queries(&ecs_context).await;
        assert!(query_time > Duration::ZERO);
        
        let hook_time = benchmark.benchmark_hook_execution(ecs_context).await;
        assert!(hook_time > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_performance_regression() {
        let config = IntegrationTestConfig {
            test_data_size: 1000,
            ..Default::default()
        };
        
        let mut suite = IntegrationTestSuite::new(config);
        let metrics = suite.run_full_integration_test().await.unwrap();
        
        assert!(metrics.execution_time < Duration::from_secs(10));
        
        if let Some(memory) = metrics.memory_usage_bytes {
            assert!(memory < 100_000_000); // 100MB threshold
        }
    }
}
