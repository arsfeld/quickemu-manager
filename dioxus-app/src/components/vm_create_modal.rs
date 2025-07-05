use dioxus::prelude::*;

use crate::models::CreateVMRequest;
use crate::server_functions::{
    cleanup_vm_creation_logs, create_vm_with_output, get_available_os, get_os_editions,
    get_popular_os, get_vm_creation_logs,
};

/// OS Selection Step Component - macOS Finder-style column view
#[component]
fn OSSelectionStep(
    search_query: Signal<String>,
    show_all_os: Signal<bool>,
    filtered_os_list: Signal<Vec<(String, Vec<String>)>>,
    selected_os: Signal<String>,
    selected_version: Signal<String>,
    selected_edition: Signal<String>,
    available_editions: Signal<Vec<String>>,
    is_loading_os: Signal<bool>,
    loading_editions: Signal<bool>,
) -> Element {
    rsx! {
        div { class: "h-full flex flex-col",
            // Search and toggle header
            div { class: "flex-shrink-0 pb-4",
                div { class: "flex flex-col sm:flex-row gap-3 items-start sm:items-center",
                    div { class: "flex-1",
                        input {
                            class: "input-macos text-sm",
                            placeholder: "Search operating systems...",
                            value: search_query(),
                            oninput: move |e| search_query.set(e.value()),
                        }
                    }

                    label {
                        class: "flex items-center cursor-pointer text-sm",
                        input {
                            r#type: "checkbox",
                            class: "mr-2",
                            checked: show_all_os(),
                            onchange: move |e| show_all_os.set(e.value() == "true"),
                        }
                        span { class: "text-gray-700", "Show all OS" }
                    }
                }
            }

            // Three-column layout (like macOS Finder)
            div { class: "flex-1 flex gap-1 min-h-0",
                // Column 1: Operating Systems
                div { class: "w-1/3 flex flex-col border border-gray-300 rounded-l-md bg-white",
                    div { class: "flex-shrink-0 px-3 py-2 bg-gray-50 border-b border-gray-300",
                        h3 { class: "text-sm font-medium text-gray-900", "Operating System" }
                    }

                    if is_loading_os() {
                        div { class: "flex-1 flex items-center justify-center",
                            div { class: "text-center",
                                div { class: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto mb-2" }
                                p { class: "text-xs text-gray-600", "Loading..." }
                            }
                        }
                    } else {
                        {
                            let os_list = filtered_os_list().clone();

                            rsx! {
                                div { class: "flex-1 overflow-y-auto",
                                    for (os_name, _versions) in os_list {
                                        {
                                            let is_selected = selected_os() == os_name;
                                            let item_class = if is_selected {
                                                "px-3 py-2 text-sm cursor-pointer bg-blue-500 text-white border-l-2 border-blue-600"
                                            } else {
                                                "px-3 py-2 text-sm cursor-pointer hover:bg-gray-100 text-gray-900 border-l-2 border-transparent"
                                            };

                                            rsx! {
                                                div {
                                                    class: "{item_class}",
                                                    onclick: move |_| {
                                                        selected_os.set(os_name.clone());
                                                        selected_version.set("".to_string());
                                                        selected_edition.set("".to_string());
                                                    },
                                                    "{os_name}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Column 2: Versions
                div { class: "w-1/3 flex flex-col border-t border-r border-b border-gray-300 bg-white",
                    div { class: "flex-shrink-0 px-3 py-2 bg-gray-50 border-b border-gray-300",
                        h3 { class: "text-sm font-medium text-gray-900", "Version" }
                    }

                    if !selected_os().is_empty() {
                        {
                            let available_versions = filtered_os_list().iter()
                                .find(|(name, _)| name == &selected_os())
                                .map(|(_, versions)| versions.clone())
                                .unwrap_or_default();

                            rsx! {
                                div { class: "flex-1 overflow-y-auto",
                                    for version in available_versions {
                                        {
                                            let is_selected = selected_version() == version;
                                            let item_class = if is_selected {
                                                "px-3 py-2 text-sm cursor-pointer bg-blue-500 text-white border-l-2 border-blue-600"
                                            } else {
                                                "px-3 py-2 text-sm cursor-pointer hover:bg-gray-100 text-gray-900 border-l-2 border-transparent"
                                            };

                                            rsx! {
                                                div {
                                                    class: "{item_class}",
                                                    onclick: move |_| {
                                                        selected_version.set(version.clone());
                                                        selected_edition.set("".to_string()); // Reset edition
                                                    },
                                                    "{version}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "flex-1 flex items-center justify-center",
                            p { class: "text-xs text-gray-500", "Select an OS" }
                        }
                    }
                }

                // Column 3: Editions
                div { class: "w-1/3 flex flex-col border border-gray-300 rounded-r-md bg-white",
                    div { class: "flex-shrink-0 px-3 py-2 bg-gray-50 border-b border-gray-300",
                        h3 { class: "text-sm font-medium text-gray-900", "Edition" }
                    }

                    if !selected_version().is_empty() {
                        if loading_editions() {
                            div { class: "flex-1 flex items-center justify-center",
                                div { class: "text-center",
                                    div { class: "animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600 mx-auto mb-2" }
                                    p { class: "text-xs text-gray-600", "Loading..." }
                                }
                            }
                        } else {
                            div { class: "flex-1 overflow-y-auto",
                                // Default edition option - only show if there are no multiple editions
                                {
                                    let editions = available_editions();
                                    let show_default = editions.len() <= 1;

                                    if show_default {
                                        let is_default_selected = selected_edition().is_empty();
                                        let default_item_class = if is_default_selected {
                                            "px-3 py-2 text-sm cursor-pointer bg-blue-500 text-white border-l-2 border-blue-600"
                                        } else {
                                            "px-3 py-2 text-sm cursor-pointer hover:bg-gray-100 text-gray-900 border-l-2 border-transparent"
                                        };

                                        rsx! {
                                            div {
                                                class: "{default_item_class}",
                                                onclick: move |_| selected_edition.set("".to_string()),
                                                "Default"
                                            }
                                        }
                                    } else {
                                        // Show helpful message when multiple editions are available
                                        rsx! {
                                            div { class: "px-3 py-2 text-xs text-gray-600 bg-yellow-50 border-l-2 border-yellow-400",
                                                "⚠️ Edition required - select from the options below"
                                            }
                                        }
                                    }
                                }

                                // Available editions
                                {
                                    let editions = available_editions().clone();

                                    rsx! {
                                        for edition in editions {
                                            {
                                                let is_selected = selected_edition() == edition;
                                                let item_class = if is_selected {
                                                    "px-3 py-2 text-sm cursor-pointer bg-blue-500 text-white border-l-2 border-blue-600"
                                                } else {
                                                    "px-3 py-2 text-sm cursor-pointer hover:bg-gray-100 text-gray-900 border-l-2 border-transparent"
                                                };

                                                rsx! {
                                                    div {
                                                        class: "{item_class}",
                                                        onclick: move |_| selected_edition.set(edition.clone()),
                                                        "{edition}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "flex-1 flex items-center justify-center",
                            p { class: "text-xs text-gray-500", "Select a version" }
                        }
                    }
                }
            }

            // Selection summary at bottom
            if !selected_os().is_empty() {
                div { class: "flex-shrink-0 mt-4 p-3 bg-blue-50 rounded-md border border-blue-200",
                    div { class: "text-sm",
                        span { class: "font-medium text-blue-900", "Selected: " }
                        span { class: "text-blue-800",
                            "{selected_os()}"
                            {if !selected_version().is_empty() { format!(" {}", selected_version()) } else { "".to_string() }}
                            {if !selected_edition().is_empty() { format!(" ({})", selected_edition()) } else { "".to_string() }}
                        }
                    }
                }
            }
        }
    }
}

/// Configuration Step Component
#[component]
fn ConfigurationStep(
    vm_name: Signal<String>,
    cpu_cores: Signal<i32>,
    ram_gb: Signal<i32>,
    disk_size_gb: Signal<i32>,
) -> Element {
    rsx! {
        div { class: "space-y-6",
            // VM Name
            div {
                label { class: "block text-sm font-medium text-gray-700 mb-2", "VM Name" }
                input {
                    class: "input-macos",
                    placeholder: "Enter VM name (optional - will auto-generate if empty)",
                    value: vm_name(),
                    oninput: move |e| vm_name.set(e.value()),
                }
                p { class: "text-xs text-gray-500 mt-1", "Leave empty to auto-generate based on OS selection" }
            }

            // CPU Configuration
            div {
                label { class: "block text-sm font-medium text-gray-700 mb-2",
                    "CPU Cores: {cpu_cores()}"
                }
                input {
                    r#type: "range",
                    class: "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer slider",
                    min: "1",
                    max: "16",
                    value: "{cpu_cores()}",
                    oninput: move |e| {
                        if let Ok(value) = e.value().parse::<i32>() {
                            cpu_cores.set(value);
                        }
                    },
                }
                div { class: "flex justify-between text-xs text-gray-500 mt-1",
                    span { "1 core" }
                    span { "16 cores" }
                }
            }

            // RAM Configuration
            div {
                label { class: "block text-sm font-medium text-gray-700 mb-2",
                    "RAM: {ram_gb()} GB"
                }
                input {
                    r#type: "range",
                    class: "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer slider",
                    min: "1",
                    max: "32",
                    value: "{ram_gb()}",
                    oninput: move |e| {
                        if let Ok(value) = e.value().parse::<i32>() {
                            ram_gb.set(value);
                        }
                    },
                }
                div { class: "flex justify-between text-xs text-gray-500 mt-1",
                    span { "1 GB" }
                    span { "32 GB" }
                }
            }

            // Disk Size Configuration
            div {
                label { class: "block text-sm font-medium text-gray-700 mb-2",
                    "Disk Size: {disk_size_gb()} GB"
                }
                input {
                    r#type: "range",
                    class: "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer slider",
                    min: "10",
                    max: "500",
                    step: "10",
                    value: "{disk_size_gb()}",
                    oninput: move |e| {
                        if let Ok(value) = e.value().parse::<i32>() {
                            disk_size_gb.set(value);
                        }
                    },
                }
                div { class: "flex justify-between text-xs text-gray-500 mt-1",
                    span { "10 GB" }
                    span { "500 GB" }
                }
            }

        }
    }
}

/// Create VM Modal Component
#[component]
pub fn CreateVMModal(show: Signal<bool>, on_create: EventHandler<()>) -> Element {
    // State management
    let mut current_step = use_signal(|| 0);
    let mut popular_os_list = use_signal(Vec::<(String, Vec<String>)>::new);
    let mut all_os_list = use_signal(Vec::<(String, Vec<String>)>::new);
    let mut filtered_os_list = use_signal(Vec::<(String, Vec<String>)>::new);
    let search_query = use_signal(|| "".to_string());
    let selected_os = use_signal(|| "".to_string());
    let selected_version = use_signal(|| "".to_string());
    let selected_edition = use_signal(|| "".to_string());
    let mut available_editions = use_signal(Vec::<String>::new);
    let show_all_os = use_signal(|| false);
    let mut is_loading_os = use_signal(|| true);
    let mut loading_editions = use_signal(|| false);

    // VM Configuration
    let vm_name = use_signal(|| "".to_string());
    let cpu_cores = use_signal(|| 2);
    let ram_gb = use_signal(|| 4);
    let disk_size_gb = use_signal(|| 20);
    let mut is_creating = use_signal(|| false);
    let mut console_output = use_signal(|| Vec::<String>::new());
    let mut show_console = use_signal(|| false);

    // Load OS lists
    use_effect(move || {
        spawn(async move {
            is_loading_os.set(true);

            if let Ok(popular_list) = get_popular_os().await {
                popular_os_list.set(popular_list.clone());
                filtered_os_list.set(popular_list);
            }

            if let Ok(all_list) = get_available_os().await {
                all_os_list.set(all_list);
            }

            is_loading_os.set(false);
        });
    });

    // Filter OS list based on search and show_all toggle
    use_effect(move || {
        let query = search_query().to_lowercase();
        let source_list = if show_all_os() {
            all_os_list()
        } else {
            popular_os_list()
        };

        if query.is_empty() {
            filtered_os_list.set(source_list);
        } else {
            let filtered: Vec<(String, Vec<String>)> = source_list
                .into_iter()
                .filter(|(os_name, _)| os_name.to_lowercase().contains(&query))
                .collect();
            filtered_os_list.set(filtered);
        }
    });

    // Load editions when OS and version are selected
    use_effect(move || {
        let os = selected_os();
        let version = selected_version();
        if !os.is_empty() && !version.is_empty() {
            spawn(async move {
                loading_editions.set(true);
                if let Ok(mut editions) = get_os_editions(os, version).await {
                    // Filter out "Default" edition if there are multiple real editions
                    if editions.len() > 1 {
                        editions.retain(|edition| edition.to_lowercase() != "default");
                    }
                    available_editions.set(editions);
                }
                loading_editions.set(false);
            });
        } else {
            available_editions.set(vec![]);
        }
    });

    rsx! {
        div {
            class: "fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4",
            onclick: move |_| if !is_creating() { show.set(false) },

            div {
                class: "modal-macos w-full max-w-4xl max-h-[90vh] overflow-hidden",
                onclick: move |e| e.stop_propagation(),

                // Simplified Header
                div { class: "border-b border-gray-200 px-6 py-4 bg-white",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-xl font-semibold text-gray-900", "Create Virtual Machine" }
                        if !is_creating() {
                            button {
                                class: "text-gray-400 hover:text-gray-600 transition-colors",
                                onclick: move |_| show.set(false),
                                svg { class: "w-5 h-5",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    fill: "none",
                                    view_box: "0 0 24 24",
                                    stroke: "currentColor",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M6 18L18 6M6 6l12 12"
                                    }
                                }
                            }
                        }
                    }

                    // Minimal step indicator
                    div { class: "flex items-center gap-2 mt-3",
                        div { class: format!("h-1 flex-1 rounded-full {}",
                            if current_step() >= 0 { "bg-blue-500" } else { "bg-gray-200" }
                        ) }
                        div { class: format!("h-1 flex-1 rounded-full {}",
                            if current_step() >= 1 { "bg-blue-500" } else { "bg-gray-200" }
                        ) }
                    }
                }

                // Content
                div { class: "p-6 h-[60vh] bg-white",
                    if show_console() {
                        // Console Output View
                        div { class: "h-full flex flex-col",

                                // Console Terminal
                                div { class: "flex-1 bg-gray-900 rounded-lg border border-gray-700 overflow-hidden flex flex-col",
                                    div { class: "bg-gray-800 px-4 py-2 border-b border-gray-700",
                                        div { class: "flex items-center space-x-2",
                                            div { class: "w-3 h-3 bg-red-500 rounded-full" }
                                            div { class: "w-3 h-3 bg-yellow-500 rounded-full" }
                                            div { class: "w-3 h-3 bg-green-500 rounded-full" }
                                            span { class: "ml-4 text-gray-300 text-sm font-mono", "quickget console" }
                                        }
                                    }
                                    div {
                                        class: "flex-1 p-4 overflow-y-auto text-sm font-mono text-green-400 bg-black",
                                        for line in console_output() {
                                            div { class: "mb-1 leading-relaxed", "{line}" }
                                        }
                                        if is_creating() {
                                            div { class: "text-yellow-400 animate-pulse inline-block", "▊" }
                                        }
                                    }
                                }
                            }
                    } else {
                        match current_step() {
                            0 => rsx! { OSSelectionStep {
                                search_query,
                                show_all_os,
                                filtered_os_list,
                                selected_os,
                                selected_version,
                                selected_edition,
                                available_editions,
                                is_loading_os,
                                loading_editions
                            } },
                            1 => rsx! { ConfigurationStep {
                                vm_name,
                                cpu_cores,
                                ram_gb,
                                disk_size_gb
                            } },
                            _ => rsx! { div {} }
                        }
                    }
                }

                // Footer with navigation
                div { class: "border-t border-gray-200 bg-gray-50 px-6 py-4 flex justify-between items-center",
                    button {
                        class: format!("btn-macos {}",
                            if current_step() == 0 { "invisible" } else { "" }
                        ),
                        disabled: current_step() == 0 || is_creating(),
                        onclick: move |_| current_step.set(current_step() - 1),
                        "Back"
                    }

                    div { class: "flex space-x-3",
                        button {
                            class: "btn-macos",
                            disabled: is_creating(),
                            onclick: move |_| {
                                show.set(false);
                                current_step.set(0);
                            },
                            "Cancel"
                        }

                        if current_step() == 0 {
                            button {
                                class: "btn-macos-primary",
                                disabled: {
                                    let os_empty = selected_os().is_empty();
                                    let version_empty = selected_version().is_empty();
                                    let editions = available_editions();
                                    let edition_required = editions.len() > 1;
                                    let edition_empty = selected_edition().is_empty();

                                    os_empty || version_empty || (edition_required && edition_empty)
                                },
                                onclick: move |_| current_step.set(current_step() + 1),
                                "Next"
                            }
                        } else if current_step() == 1 {
                            button {
                                class: "btn-macos-primary",
                                disabled: is_creating() || cpu_cores() < 1 || ram_gb() < 1 || disk_size_gb() < 1,
                                onclick: move |_| {
                                    is_creating.set(true);
                                    show_console.set(true);
                                    console_output.set(vec!["Initializing VM creation...".to_string()]);

                                    let os = selected_os();
                                    let version = selected_version();
                                    let edition = if selected_edition().is_empty() { None } else { Some(selected_edition()) };
                                    let name = if vm_name().is_empty() { None } else { Some(vm_name()) };
                                    let ram_mb = ram_gb() * 1024;
                                    let disk_gb_str = format!("{}G", disk_size_gb());


                                    spawn(async move {
                                        let request = CreateVMRequest {
                                            os: os.clone(),
                                            version: version.clone(),
                                            edition,
                                            name,
                                            ram: Some(format!("{}G", ram_mb / 1024)),
                                            disk_size: Some(disk_gb_str),
                                            cpu_cores: Some(cpu_cores() as u32),
                                        };

                                        // Start the creation process
                                        console_output.with_mut(|output| {
                                            output.push("Calling server function...".to_string());
                                        });

                                        match create_vm_with_output(request).await {
                                            Ok(creation_id) => {
                                                console_output.with_mut(|output| {
                                                    output.push(format!("Got creation ID: {}", creation_id));
                                                });
                                                // Start polling for logs in real-time

                                                loop {
                                                    // Poll for new logs
                                                    match get_vm_creation_logs(creation_id.clone()).await {
                                                        Ok(logs) => {
                                                            console_output.with_mut(|output| {
                                                                output.push(format!("Polled {} log lines", logs.len()));
                                                            });

                                                            // Always update console with all logs from server
                                                            if !logs.is_empty() && logs[0] != "Creation process not found" {
                                                                console_output.set(logs.clone());
                                                            } else {
                                                                console_output.with_mut(|output| {
                                                                    if output.len() < 6 {
                                                                        output.push(format!("No logs found for ID: {}", creation_id));
                                                                    }
                                                                });
                                                            }

                                                            // Check if process is complete (success or error message)
                                                            if let Some(last_line) = logs.last() {
                                                                if last_line.starts_with("✓") || last_line.starts_with("✗") {
                                                                    // Process completed
                                                                    let success = last_line.starts_with("✓");

                                                                    // Clean up logs
                                                                    let _ = cleanup_vm_creation_logs(creation_id.clone()).await;

                                                                    if success {
                                                                        on_create.call(());
                                                                        show.set(false);
                                                                        current_step.set(0);
                                                                        show_console.set(false);
                                                                    }
                                                                    is_creating.set(false);
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            console_output.with_mut(|output| {
                                                                output.push(format!("✗ Error polling logs: {}", e));
                                                            });
                                                            is_creating.set(false);
                                                            break;
                                                        }
                                                    }

                                                    // Wait before next poll (web-compatible)
                                                    #[cfg(target_arch = "wasm32")]
                                                    gloo_timers::future::TimeoutFuture::new(500).await;

                                                    #[cfg(not(target_arch = "wasm32"))]
                                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                                }
                                            }
                                            Err(e) => {
                                                console_output.with_mut(|output| {
                                                    output.push(format!("✗ Error starting VM creation: {}", e));
                                                });
                                                is_creating.set(false);
                                            }
                                        }
                                    });
                                },
                                if is_creating() { "Creating..." } else { "Create VM" }
                            }
                        }
                    }
                }
            }
        }
    }
}
