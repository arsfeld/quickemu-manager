use dioxus::prelude::*;
use crate::models::{VM, VMStatus, VMMetrics, MetricsHistory};

#[component]
pub fn VMDetail(
    vm: VM,
    metrics: Option<VMMetrics>,
    history: Option<MetricsHistory>,
    on_start: EventHandler<()>,
    on_stop: EventHandler<()>,
    on_restart: EventHandler<()>,
    on_console: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    let metrics = metrics.as_ref();
    
    rsx! {
        div { class: "vm-detail",
            div { class: "vm-detail-header",
                h2 { "{vm.name}" }
                button {
                    class: "btn-close",
                    onclick: move |_| on_close(()),
                    "✕"
                }
            }
            
            div { class: "vm-detail-content",
                div { class: "vm-detail-section",
                    h3 { "Configuration" }
                    div { class: "config-grid",
                        div { "Operating System:" }
                        div { "{vm.config.guest_os}" }
                        
                        div { "CPU Cores:" }
                        div { "{vm.config.cpu_cores}" }
                        
                        div { "Memory:" }
                        div { "{vm.config.ram}" }
                        
                        if let Some(disk_size) = &vm.config.disk_size {
                            div { "Disk Size:" }
                            div { "{disk_size}" }
                        }
                        
                        div { "Display:" }
                        div { "{vm.config.display:?}" }
                    }
                }
                
                if let Some(metrics) = metrics {
                    div { class: "vm-detail-section",
                        h3 { "Performance" }
                        div { class: "metrics-grid",
                            div { class: "metric",
                                span { class: "metric-label", "CPU Usage" }
                                span { class: "metric-value", "{metrics.cpu_percent:.1}%" }
                            }
                            div { class: "metric",
                                span { class: "metric-label", "Memory" }
                                span { class: "metric-value", "{metrics.memory_mb} MB ({metrics.memory_percent:.1}%)" }
                            }
                        }
                    }
                }
                
                div { class: "vm-detail-actions",
                    match &vm.status {
                        VMStatus::Stopped => rsx! {
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| on_start(()),
                                "Start VM"
                            }
                        },
                        VMStatus::Running { .. } => rsx! {
                            button {
                                class: "btn btn-danger",
                                onclick: move |_| on_stop(()),
                                "Stop VM"
                            }
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| on_restart(()),
                                "Restart VM"
                            }
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| on_console(()),
                                "Open Console"
                            }
                        },
                        _ => rsx! { div {} }
                    }
                }
            }
        }
    }
}