//! 

use crate::prelude::*;
use bevy_ecs::prelude::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub ecs_query_times: Vec<Duration>,
    pub hook_execution_times: Vec<Duration>,
    pub render_times: Vec<Duration>,
    pub memory_snapshots: Vec<MemorySnapshot>,
    pub total_operations: usize,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            ecs_query_times: Vec::new(),
            hook_execution_times: Vec::new(),
            render_times: Vec::new(),
            memory_snapshots: Vec::new(),
            total_operations: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub timestamp: Instant,
    pub heap_size: u64,
    pub stack_size: u64,
    pub ecs_world_size: u64,
}

pub struct PerformanceProfiler {
    metrics: Arc<Mutex<PerformanceMetrics>>,
    enabled: bool,
    sample_rate: f32,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(PerformanceMetrics::default())),
            enabled: true,
            sample_rate: 1.0,
        }
    }

    pub fn with_sample_rate(mut self, rate: f32) -> Self {
        self.sample_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn should_sample(&self) -> bool {
        if !self.enabled {
            return false;
        }
        fastrand::f32() < self.sample_rate
    }

    pub fn record_ecs_query_time(&self, duration: Duration) {
        if self.should_sample() {
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.ecs_query_times.push(duration);
                metrics.total_operations += 1;
            }
        }
    }

    pub fn record_hook_execution_time(&self, duration: Duration) {
        if self.should_sample() {
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.hook_execution_times.push(duration);
                metrics.total_operations += 1;
            }
        }
    }

    pub fn record_render_time(&self, duration: Duration) {
        if self.should_sample() {
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.render_times.push(duration);
                metrics.total_operations += 1;
            }
        }
    }

    pub fn take_memory_snapshot(&self, ecs_context: &EcsWorldContext) {
        if self.should_sample() {
            let snapshot = MemorySnapshot {
                timestamp: Instant::now(),
                heap_size: self.estimate_heap_size(),
                stack_size: self.estimate_stack_size(),
                ecs_world_size: self.estimate_ecs_world_size(ecs_context),
            };

            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.memory_snapshots.push(snapshot);
            }
        }
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.lock().unwrap().clone()
    }

    pub fn reset_metrics(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            *metrics = PerformanceMetrics::default();
        }
    }

    fn estimate_heap_size(&self) -> u64 {
        1024 * 1024 // 1MB placeholder
    }

    fn estimate_stack_size(&self) -> u64 {
        64 * 1024 // 64KB placeholder
    }

    fn estimate_ecs_world_size(&self, _ecs_context: &EcsWorldContext) -> u64 {
        512 * 1024 // 512KB placeholder
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PerformanceAnalyzer;

impl PerformanceAnalyzer {
    pub fn analyze_metrics(metrics: &PerformanceMetrics) -> PerformanceReport {
        let mut report = PerformanceReport::default();

        if !metrics.ecs_query_times.is_empty() {
            report.ecs_stats = Some(Self::analyze_durations(&metrics.ecs_query_times));
        }

        if !metrics.hook_execution_times.is_empty() {
            report.hook_stats = Some(Self::analyze_durations(&metrics.hook_execution_times));
        }

        if !metrics.render_times.is_empty() {
            report.render_stats = Some(Self::analyze_durations(&metrics.render_times));
        }

        if !metrics.memory_snapshots.is_empty() {
            report.memory_stats = Some(Self::analyze_memory(&metrics.memory_snapshots));
        }

        report.total_operations = metrics.total_operations;
        report.recommendations = Self::generate_recommendations(&report);

        report
    }

    fn analyze_durations(durations: &[Duration]) -> DurationStats {
        let mut sorted = durations.to_vec();
        sorted.sort();

        let total: Duration = sorted.iter().sum();
        let count = sorted.len();
        let mean = total / count as u32;

        let median = if count % 2 == 0 {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2
        } else {
            sorted[count / 2]
        };

        let p95_index = (count as f64 * 0.95) as usize;
        let p95 = sorted.get(p95_index).copied().unwrap_or(Duration::ZERO);

        let p99_index = (count as f64 * 0.99) as usize;
        let p99 = sorted.get(p99_index).copied().unwrap_or(Duration::ZERO);

        DurationStats {
            mean,
            median,
            p95,
            p99,
            min: sorted.first().copied().unwrap_or(Duration::ZERO),
            max: sorted.last().copied().unwrap_or(Duration::ZERO),
            total,
            count,
        }
    }

    fn analyze_memory(snapshots: &[MemorySnapshot]) -> MemoryStats {
        let heap_sizes: Vec<u64> = snapshots.iter().map(|s| s.heap_size).collect();
        let stack_sizes: Vec<u64> = snapshots.iter().map(|s| s.stack_size).collect();
        let ecs_sizes: Vec<u64> = snapshots.iter().map(|s| s.ecs_world_size).collect();

        MemoryStats {
            peak_heap: heap_sizes.iter().max().copied().unwrap_or(0),
            avg_heap: heap_sizes.iter().sum::<u64>() / heap_sizes.len() as u64,
            peak_stack: stack_sizes.iter().max().copied().unwrap_or(0),
            avg_stack: stack_sizes.iter().sum::<u64>() / stack_sizes.len() as u64,
            peak_ecs_world: ecs_sizes.iter().max().copied().unwrap_or(0),
            avg_ecs_world: ecs_sizes.iter().sum::<u64>() / ecs_sizes.len() as u64,
            snapshot_count: snapshots.len(),
        }
    }

    fn generate_recommendations(report: &PerformanceReport) -> Vec<String> {
        let mut recommendations = Vec::new();

        if let Some(ref ecs_stats) = report.ecs_stats {
            if ecs_stats.p95 > Duration::from_millis(10) {
                recommendations.push("Consider optimizing ECS queries - P95 latency is high".to_string());
            }
            if ecs_stats.mean > Duration::from_millis(5) {
                recommendations.push("ECS query mean time is elevated - review query complexity".to_string());
            }
        }

        if let Some(ref hook_stats) = report.hook_stats {
            if hook_stats.p95 > Duration::from_millis(16) {
                recommendations.push("Hook execution time may impact 60fps rendering".to_string());
            }
        }

        if let Some(ref render_stats) = report.render_stats {
            if render_stats.mean > Duration::from_millis(16) {
                recommendations.push("Render time exceeds 60fps budget - optimize rendering".to_string());
            }
        }

        if let Some(ref memory_stats) = report.memory_stats {
            if memory_stats.peak_heap > 100 * 1024 * 1024 {
                recommendations.push("High memory usage detected - consider memory optimization".to_string());
            }
        }

        if recommendations.is_empty() {
            recommendations.push("Performance looks good!".to_string());
        }

        recommendations
    }
}

#[derive(Debug, Clone, Default)]
pub struct PerformanceReport {
    pub ecs_stats: Option<DurationStats>,
    pub hook_stats: Option<DurationStats>,
    pub render_stats: Option<DurationStats>,
    pub memory_stats: Option<MemoryStats>,
    pub total_operations: usize,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DurationStats {
    pub mean: Duration,
    pub median: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub min: Duration,
    pub max: Duration,
    pub total: Duration,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub peak_heap: u64,
    pub avg_heap: u64,
    pub peak_stack: u64,
    pub avg_stack: u64,
    pub peak_ecs_world: u64,
    pub avg_ecs_world: u64,
    pub snapshot_count: usize,
}

pub struct ProfiledEcsContext {
    inner: EcsWorldContext,
    profiler: PerformanceProfiler,
}

impl ProfiledEcsContext {
    pub fn new(ecs_context: EcsWorldContext) -> Self {
        Self {
            inner: ecs_context,
            profiler: PerformanceProfiler::new(),
        }
    }

    pub fn with_profiler(ecs_context: EcsWorldContext, profiler: PerformanceProfiler) -> Self {
        Self {
            inner: ecs_context,
            profiler,
        }
    }

    pub fn run_system<S, Out, Marker>(&self, system: S) -> Result<Out, crate::EcsContextError>
    where
        S: IntoSystem<(), Out, Marker> + 'static,
        Out: 'static,
    {
        let start = Instant::now();
        let result = self.inner.run_system(system);
        let duration = start.elapsed();
        
        self.profiler.record_ecs_query_time(duration);
        self.profiler.take_memory_snapshot(&self.inner);
        
        result
    }

    pub fn get_profiler(&self) -> &PerformanceProfiler {
        &self.profiler
    }

    pub fn get_inner(&self) -> &EcsWorldContext {
        &self.inner
    }

    pub fn generate_report(&self) -> PerformanceReport {
        let metrics = self.profiler.get_metrics();
        PerformanceAnalyzer::analyze_metrics(&metrics)
    }
}

pub struct OptimizationHints;

impl OptimizationHints {
    pub fn suggest_query_optimizations(query_count: usize, avg_time: Duration) -> Vec<String> {
        let mut suggestions = Vec::new();

        if query_count > 1000 && avg_time > Duration::from_millis(1) {
            suggestions.push("Consider batching ECS queries to reduce overhead".to_string());
        }

        if avg_time > Duration::from_millis(5) {
            suggestions.push("Review query filters and component access patterns".to_string());
        }

        suggestions
    }

    pub fn suggest_memory_optimizations(peak_memory: u64) -> Vec<String> {
        let mut suggestions = Vec::new();

        if peak_memory > 50 * 1024 * 1024 {
            suggestions.push("Consider implementing component pooling".to_string());
            suggestions.push("Review entity lifecycle management".to_string());
        }

        suggestions
    }

    pub fn suggest_render_optimizations(render_time: Duration) -> Vec<String> {
        let mut suggestions = Vec::new();

        if render_time > Duration::from_millis(16) {
            suggestions.push("Consider reducing DOM mutations per frame".to_string());
            suggestions.push("Implement virtual scrolling for large lists".to_string());
        }

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;

    #[derive(Component)]
    struct TestComponent(i32);

    #[derive(Resource)]
    struct TestResource(i32);

    #[test]
    fn test_performance_profiler() {
        let profiler = PerformanceProfiler::new();
        
        profiler.record_ecs_query_time(Duration::from_millis(5));
        profiler.record_hook_execution_time(Duration::from_millis(2));
        profiler.record_render_time(Duration::from_millis(16));
        
        let metrics = profiler.get_metrics();
        assert_eq!(metrics.ecs_query_times.len(), 1);
        assert_eq!(metrics.hook_execution_times.len(), 1);
        assert_eq!(metrics.render_times.len(), 1);
        assert_eq!(metrics.total_operations, 3);
    }

    #[test]
    fn test_performance_analyzer() {
        let mut metrics = PerformanceMetrics::default();
        metrics.ecs_query_times = vec![
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(3),
            Duration::from_millis(4),
            Duration::from_millis(5),
        ];
        
        let report = PerformanceAnalyzer::analyze_metrics(&metrics);
        
        assert!(report.ecs_stats.is_some());
        let ecs_stats = report.ecs_stats.unwrap();
        assert_eq!(ecs_stats.count, 5);
        assert_eq!(ecs_stats.median, Duration::from_millis(3));
        assert_eq!(ecs_stats.min, Duration::from_millis(1));
        assert_eq!(ecs_stats.max, Duration::from_millis(5));
    }

    #[test]
    fn test_profiled_ecs_context() {
        let mut world = World::new();
        world.insert_resource(TestResource(42));
        world.spawn(TestComponent(1));
        
        let ecs_context = EcsWorldContext::new(world);
        let profiled_context = ProfiledEcsContext::new(ecs_context);
        
        let test_system = |res: Res<TestResource>| res.0;
        let result = profiled_context.run_system(test_system).unwrap();
        assert_eq!(result, 42);
        
        let metrics = profiled_context.get_profiler().get_metrics();
        assert!(metrics.total_operations > 0);
    }

    #[test]
    fn test_optimization_hints() {
        let query_hints = OptimizationHints::suggest_query_optimizations(2000, Duration::from_millis(10));
        assert!(!query_hints.is_empty());
        
        let memory_hints = OptimizationHints::suggest_memory_optimizations(100 * 1024 * 1024);
        assert!(!memory_hints.is_empty());
        
        let render_hints = OptimizationHints::suggest_render_optimizations(Duration::from_millis(20));
        assert!(!render_hints.is_empty());
    }
}
