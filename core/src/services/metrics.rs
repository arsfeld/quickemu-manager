use crate::models::{VMId, VMMetrics, MetricsHistory};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct MetricsService {
    histories: Arc<RwLock<HashMap<VMId, MetricsHistory>>>,
    max_history_points: usize,
}

impl MetricsService {
    pub fn new(max_history_points: usize) -> Self {
        Self {
            histories: Arc::new(RwLock::new(HashMap::new())),
            max_history_points,
        }
    }
    
    pub async fn record_metrics(&self, vm_id: VMId, metrics: VMMetrics) {
        let mut histories = self.histories.write().await;
        
        let history = histories
            .entry(vm_id)
            .or_insert_with(|| MetricsHistory::new(self.max_history_points));
        
        history.add_sample(&metrics);
    }
    
    pub async fn get_history(&self, vm_id: &VMId) -> Option<MetricsHistory> {
        self.histories.read().await.get(vm_id).cloned()
    }
    
    pub async fn get_all_histories(&self) -> HashMap<VMId, MetricsHistory> {
        self.histories.read().await.clone()
    }
    
    pub async fn clear_history(&self, vm_id: &VMId) {
        self.histories.write().await.remove(vm_id);
    }
    
    pub async fn clear_all_histories(&self) {
        self.histories.write().await.clear();
    }
}