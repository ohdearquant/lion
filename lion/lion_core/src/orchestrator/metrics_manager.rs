use tokio::sync::RwLock;
use std::sync::Arc;
use tokio::time::Instant;

/// Metrics for monitoring orchestrator performance
#[derive(Debug, Default, Clone)]
pub struct OrchestratorMetrics {
    pub active_agents: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub messages_processed: usize,
    pub average_processing_time: f64,
}

/// Manages metrics collection and updates
pub struct MetricsManager {
    metrics: Arc<RwLock<OrchestratorMetrics>>,
}

impl MetricsManager {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(OrchestratorMetrics::default())),
        }
    }

    /// Update processing metrics
    pub async fn update_processing_metrics(&self, processing_time: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.messages_processed += 1;
        metrics.average_processing_time = (metrics.average_processing_time * (metrics.messages_processed - 1) as f64
            + processing_time) / metrics.messages_processed as f64;
    }

    /// Increment completed tasks
    pub async fn increment_completed_tasks(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.completed_tasks += 1;
    }

    /// Increment failed tasks
    pub async fn increment_failed_tasks(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.failed_tasks += 1;
    }

    /// Get current metrics snapshot
    pub async fn get_metrics(&self) -> OrchestratorMetrics {
        (*self.metrics.read().await).clone()
    }

    /// Start timing an operation
    pub fn start_timing() -> Instant {
        Instant::now()
    }

    /// End timing and get duration in seconds
    pub fn end_timing(start: Instant) -> f64 {
        start.elapsed().as_secs_f64()
    }
}

impl Clone for MetricsManager {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics.clone(),
        }
    }
}
