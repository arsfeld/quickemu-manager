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
                MiniGraph { metrics: current_metrics, metric_type: metric_type.clone() }
            } else if is_loading() {
                div {
                    class: "flex flex-col items-center justify-center h-full bg-gradient-to-br from-blue-50 to-indigo-50 rounded-xl border border-blue-200 shadow-sm p-3",
                    div {
                        class: "w-6 h-6 border-2 border-blue-200 border-t-blue-500 rounded-full animate-spin mb-2"
                    }
                    span {
                        class: "text-xs font-medium text-blue-600",
                        "Loading metrics..."
                    }
                }
            } else {
                div {
                    class: "flex flex-col items-center justify-center h-full bg-gradient-to-br from-slate-50 to-slate-100 rounded-xl border border-slate-200 shadow-sm p-3",
                    div {
                        class: "w-8 h-8 rounded-full bg-gray-100 border border-gray-200 mb-2 flex items-center justify-center",
                        svg {
                            class: "w-4 h-4 text-gray-400",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            stroke_width: "2",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                d: "M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"
                            }
                        }
                    }
                    span {
                        class: "text-xs font-medium text-gray-500",
                        "VM Stopped"
                    }
                    span {
                        class: "text-xs text-gray-400 mt-1",
                        "No metrics available"
                    }
                }
            }
        }
    }
}

#[component]
fn MiniGraph(metrics: VMMetrics, metric_type: String) -> Element {
    let (label, value, color, svg_content) = match metric_type.as_str() {
        "cpu" => (
            "CPU",
            format!("{:.0}%", metrics.cpu_percent),
            get_metric_color(metrics.cpu_percent, 80.0),
            create_mini_graph_path(metrics.cpu_percent, 100.0)
        ),
        "memory" => (
            "RAM", 
            format!("{:.0}%", metrics.memory_percent),
            get_metric_color(metrics.memory_percent, 85.0),
            create_mini_graph_path(metrics.memory_percent, 100.0)
        ),
        "network_rx" => (
            "Download",
            format_bytes_compact(metrics.network_rx_bytes),
            "#3b82f6", // Apple-like blue
            create_network_mini_graph(metrics.network_rx_bytes)
        ),
        "network_tx" => (
            "Upload", 
            format_bytes_compact(metrics.network_tx_bytes),
            "#8b5cf6", // Apple-like purple
            create_network_mini_graph(metrics.network_tx_bytes)
        ),
        _ => ("N/A", "0".to_string(), "#gray", String::new())
    };

    rsx! {
        div {
            class: "flex flex-col items-center justify-center text-center w-full h-full bg-gradient-to-br from-white to-gray-50 rounded-xl border border-gray-200 shadow-sm p-3 hover:shadow-md transition-all duration-200",
            
            // Big Title with modern styling
            h3 {
                class: "text-sm font-semibold mb-2 tracking-wide",
                style: format!("color: {};", color),
                "{label}"
            }
            
            // Graph in the middle with Apple-like styling
            div {
                class: "w-full h-10 mb-3 flex items-center justify-center",
                svg {
                    width: "90%",
                    height: "100%",
                    view_box: "0 0 90 40",
                    
                    // Background with subtle shadow
                    rect {
                        x: "5",
                        y: "30",
                        width: "80",
                        height: "6",
                        rx: "3",
                        fill: "#f1f5f9",
                        stroke: "#e2e8f0",
                        stroke_width: "0.5"
                    }
                    
                    // Data visualization
                    if !svg_content.is_empty() {
                        path {
                            d: "{svg_content}",
                            fill: "none",
                            stroke: "{color}",
                            stroke_width: "2.5",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            opacity: "0.8"
                        }
                    }
                    
                    // Progress bar for percentage metrics with gradient
                    if metric_type == "cpu" || metric_type == "memory" {
                        defs {
                            linearGradient {
                                id: format!("gradient-{}", metric_type),
                                x1: "0%",
                                y1: "0%",
                                x2: "100%",
                                y2: "0%",
                                stop {
                                    offset: "0%",
                                    stop_color: "{color}",
                                    stop_opacity: "0.8"
                                }
                                stop {
                                    offset: "100%",
                                    stop_color: "{color}",
                                    stop_opacity: "1.0"
                                }
                            }
                        }
                        rect {
                            x: "5",
                            y: "30",
                            width: format!("{:.1}", 
                                if metric_type == "cpu" {
                                    (metrics.cpu_percent.min(100.0) / 100.0) * 80.0
                                } else {
                                    (metrics.memory_percent.min(100.0) / 100.0) * 80.0
                                }
                            ),
                            height: "6",
                            rx: "3",
                            fill: format!("url(#gradient-{})", metric_type)
                        }
                    }
                }
            }
            
            // Big Value with enhanced typography
            span {
                class: "text-lg font-bold font-mono tracking-tight",
                style: format!("color: {};", color),
                "{value}"
            }
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