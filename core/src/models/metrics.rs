use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VMMetrics {
    pub cpu_percent: f32,
    pub memory_mb: u32,
    pub memory_percent: f32,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricsHistory {
    pub cpu: VecDeque<f32>,
    pub memory: VecDeque<f32>,
    pub max_points: usize,
}

impl MetricsHistory {
    pub fn new(max_points: usize) -> Self {
        Self {
            cpu: VecDeque::with_capacity(max_points),
            memory: VecDeque::with_capacity(max_points),
            max_points,
        }
    }
    
    pub fn add_sample(&mut self, metrics: &VMMetrics) {
        if self.cpu.len() >= self.max_points {
            self.cpu.pop_front();
        }
        if self.memory.len() >= self.max_points {
            self.memory.pop_front();
        }
        
        self.cpu.push_back(metrics.cpu_percent);
        self.memory.push_back(metrics.memory_percent);
    }
    
    pub fn cpu_average(&self) -> f32 {
        if self.cpu.is_empty() {
            0.0
        } else {
            self.cpu.iter().sum::<f32>() / self.cpu.len() as f32
        }
    }
    
    pub fn memory_average(&self) -> f32 {
        if self.memory.is_empty() {
            0.0
        } else {
            self.memory.iter().sum::<f32>() / self.memory.len() as f32
        }
    }
}