use dioxus::prelude::*;
use crate::models::VMMetrics;
use crate::server_functions::get_vm_metrics;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn VMMetricsCard(vm_id: String, vm_name: String, metric_type: String) -> Element {
    let mut metrics = use_signal(|| None::<VMMetrics>);
    let mut is_loading = use_signal(|| true);

    // Real-time polling for current metrics (every 3 seconds)
    use_effect({
        let vm_id = vm_id.clone();
        move || {
            let vm_id = vm_id.clone();
            spawn(async move {
                loop {
                    match get_vm_metrics(vm_id.clone()).await {
                        Ok(current_metrics) => {
                            metrics.set(Some(current_metrics));
                            is_loading.set(false);
                        }
                        Err(_) => {
                            is_loading.set(false);
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    TimeoutFuture::new(3000).await;
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
                }
            });
        }
    });

    rsx! {
        div {
            class: "w-full h-full flex flex-col items-center justify-center",
            
            if let Some(current_metrics) = metrics() {
                // Match the parent component's styling
                div { class: "text-center",
                    MiniGraph { metrics: current_metrics, metric_type: metric_type.clone() }
                }
            } else if is_loading() {
                // Loading state matching the design
                div { class: "text-center",
                    div { class: "text-2xl font-bold text-slate-100 mb-1 animate-pulse", "..." }
                    span { class: "text-xs text-slate-400 uppercase tracking-wide", "Loading" }
                }
            } else {
                // Error/Stopped state matching the design
                div { class: "text-center",
                    div { class: "text-2xl font-bold text-slate-100 mb-1", "--" }
                    span { class: "text-xs text-slate-400 uppercase tracking-wide", "Offline" }
                }
            }
        }
    }
}

#[component]
fn MiniGraph(metrics: VMMetrics, metric_type: String) -> Element {
    let (label, value, unit) = match metric_type.as_str() {
        "cpu" => (
            "CPU",
            format!("{:.0}", metrics.cpu_percent),
            "%"
        ),
        "memory" => (
            "RAM", 
            format!("{:.0}", metrics.memory_percent),
            "%"
        ),
        "network_rx" => (
            "RX",
            format_bytes_compact(metrics.network_rx_bytes),
            "/s"
        ),
        "network_tx" => (
            "TX", 
            format_bytes_compact(metrics.network_tx_bytes),
            "/s"
        ),
        _ => ("N/A", "0".to_string(), "")
    };

    rsx! {
        // Match the static block design exactly
        div { 
            class: "text-2xl font-bold text-slate-100 mb-1",
            "{value}"
            if !unit.is_empty() {
                span { class: "text-lg text-slate-300", "{unit}" }
            }
        }
        span { 
            class: "text-xs text-slate-400 uppercase tracking-wide",
            "{label}"
        }
    }
}

// Helper functions
fn get_metric_color(value: f32, threshold: f32) -> &'static str {
    if value > threshold {
        "#ef4444" // Apple-like red for high usage
    } else if value > threshold * 0.6 {
        "#f59e0b" // Apple-like amber for medium usage
    } else {
        "#10b981" // Apple-like green for low usage
    }
}

fn format_bytes_compact(bytes: u64) -> String {
    if bytes == 0 {
        return "0".to_string();
    }
    
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{:.0}{}", size, UNITS[unit_index])
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}

fn create_mini_graph_path(current_value: f32, max_value: f32) -> String {
    // Create a simple sine wave pattern based on current value
    let amplitude = (current_value / max_value) * 8.0; // Max 8px amplitude
    let frequency = 0.15;
    
    let mut path = "M 8 16".to_string();
    for i in 1..=64 {
        let x = 8.0 + i as f32;
        let y = 16.0 + amplitude * (frequency * x).sin();
        path.push_str(&format!(" L {:.1} {:.1}", x, y));
    }
    
    path
}

fn create_network_mini_graph(bytes: u64) -> String {
    // Create activity spikes based on network usage
    if bytes == 0 {
        return String::new();
    }
    
    let intensity = (bytes as f32).log10().min(6.0) / 6.0; // Normalize to 0-1
    let spike_height = intensity * 10.0;
    
    let mut path = "M 8 20".to_string();
    for i in 1..=16 {
        let x = 8.0 + i as f32 * 4.0; // Every 4 pixels
        let variation = ((i as f32 * 0.8).sin() * 0.5 + 0.5) * spike_height;
        let y = 20.0 - variation;
        path.push_str(&format!(" L {:.1} {:.1}", x, y));
    }
    
    path
}