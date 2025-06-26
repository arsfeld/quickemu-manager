use dioxus::prelude::*;

use crate::models::VM;
use crate::server_functions::get_vm_screenshot;

/// VM Screen Modal Component
#[component]
pub fn VMScreenModal(vm: VM, on_close: EventHandler<()>) -> Element {
    let mut screenshot_data = use_signal(|| None::<String>);
    let mut error_message = use_signal(|| None::<String>);
    let mut is_loading = use_signal(|| true);
    let vm_id = vm.id.clone();

    // Fetch screenshot on mount and then periodically
    use_effect(move || {
        let vm_id_clone = vm_id.clone();
        spawn(async move {
            loop {
                is_loading.set(true);
                match get_vm_screenshot(vm_id_clone.clone()).await {
                    Ok(data) => {
                        screenshot_data.set(Some(data));
                        error_message.set(None);
                    },
                    Err(e) => {
                        error_message.set(Some(format!("Failed to get screenshot: {}", e)));
                    }
                }
                is_loading.set(false);

                // Refresh every 2 seconds
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(2000).await;
                
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
            }
        });
    });

    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-50",
            onclick: move |_| on_close.call(()),

            div {
                class: "modal-macos p-0 max-w-4xl w-full",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex items-center justify-between p-4 border-b border-macos-border",
                    h2 { class: "text-xl font-semibold", "VM Screen: {vm.name}" }
                    button {
                        class: "text-gray-500 hover:text-gray-800 transition-colors text-2xl",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }

                // Screen content
                div {
                    class: "p-4 bg-black",
                    if is_loading() && screenshot_data().is_none() {
                        div {
                            class: "flex flex-col items-center justify-center h-96",
                            div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" }
                            p { class: "text-white mt-4", "Loading screen..." }
                        }
                    } else if let Some(error) = error_message() {
                        div {
                            class: "flex flex-col items-center justify-center h-96 bg-red-100 text-red-800 p-4 rounded-lg",
                            p { "{error}" }
                        }
                    } else if let Some(data) = screenshot_data() {
                        img {
                            src: "{data}",
                            class: "w-full h-auto object-contain rounded-lg"
                        }
                    }
                }
                
                // Footer
                div {
                    class: "p-4 bg-gray-50 border-t border-macos-border flex items-center justify-between",
                    div {
                        class: "flex items-center space-x-2 text-sm text-gray-600",
                        if is_loading() {
                            div { class: "w-2 h-2 bg-blue-500 rounded-full animate-pulse" }
                            span { "Updating..." }
                        } else {
                            div { class: "w-2 h-2 bg-green-500 rounded-full" }
                            span { "Live (refreshes every 2s)" }
                        }
                    }
                    button {
                        class: "btn-macos",
                        onclick: move |_| on_close.call(()),
                        "Close"
                    }
                }
            }
        }
    }
}
