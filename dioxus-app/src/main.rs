use dioxus::prelude::*;

mod models;
mod server_functions;

use crate::models::{VM, VMStatus, CreateVMRequest, VMStatusExt};
use crate::server_functions::*;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Script { src: "https://cdn.tailwindcss.com" }
        Router::<Route> {}
    }
}

#[component]
pub fn Hero() -> Element {
    rsx! {
        div {
            id: "hero",
            img { src: HEADER_SVG, id: "header" }
            div { id: "links",
                a { href: "https://dioxuslabs.com/learn/0.6/", "📚 Learn Dioxus" }
                a { href: "https://dioxuslabs.com/awesome", "🚀 Awesome Dioxus" }
                a { href: "https://github.com/dioxus-community/", "📡 Community Libraries" }
                a { href: "https://github.com/DioxusLabs/sdk", "⚙️ Dioxus Development Kit" }
                a { href: "https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus", "💫 VSCode Extension" }
                a { href: "https://discord.gg/XgGxMSkvUM", "👋 Community Discord" }
            }
        }
    }
}

/// Home page - VM Management Dashboard
#[component]
fn Home() -> Element {
    let mut vms = use_signal(Vec::<VM>::new);
    let mut show_create_modal = use_signal(|| false);
    
    // Load VMs on component mount
    use_effect(move || {
        spawn(async move {
            if let Ok(vm_list) = get_vms().await {
                vms.set(vm_list);
            }
        });
    });
    
    rsx! {
        div { class: "container mx-auto px-4 py-8",
            // Header
            div { class: "flex items-center justify-between mb-8",
                h1 { class: "text-3xl font-bold text-gray-800", "Virtual Machines" }
                button {
                    class: "bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2 px-4 rounded-lg shadow-md transition-colors duration-200 flex items-center space-x-2",
                    onclick: move |_| show_create_modal.set(true),
                    span { "➕" }
                    span { "Create VM" }
                }
            }
            
            // VM Grid
            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
                for vm in vms().iter() {
                    VMCard { vm: vm.clone() }
                }
            }
            
            // Create VM Modal
            if show_create_modal() {
                CreateVMModal { 
                    show: show_create_modal,
                    on_create: move |_| {
                        // Refresh VM list after creation
                        spawn(async move {
                            if let Ok(vm_list) = get_vms().await {
                                vms.set(vm_list);
                            }
                        });
                    }
                }
            }
        }
    }
}

/// Blog page
#[component]
pub fn Blog(id: i32) -> Element {
    rsx! {
        div {
            id: "blog",

            // Content
            h1 { "This is blog #{id}!" }
            p { "In blog #{id}, we show how the Dioxus router works and how URL parameters can be passed as props to our route components." }

            // Navigation links
            Link {
                to: Route::Blog { id: id - 1 },
                "Previous"
            }
            span { " <---> " }
            Link {
                to: Route::Blog { id: id + 1 },
                "Next"
            }
        }
    }
}

/// Shared navbar component.
#[component]
fn Navbar() -> Element {
    rsx! {
        div {
            class: "bg-gray-900 text-white shadow-lg",
            div { class: "container mx-auto px-4 py-4 flex items-center justify-between",
                div { class: "flex items-center space-x-2",
                    h2 { class: "text-2xl font-bold", "🖥️ Quickemu Manager" }
                }
                div { class: "flex space-x-6",
                    Link {
                        to: Route::Home {},
                        class: "hover:text-blue-400 transition-colors",
                        "VMs"
                    }
                    Link {
                        to: Route::Blog { id: 1 },
                        class: "hover:text-blue-400 transition-colors",
                        "About"
                    }
                }
            }
        }

        div { class: "min-h-screen bg-gray-50",
            Outlet::<Route> {}
        }
    }
}

/// Echo component that demonstrates fullstack server functions.
#[component]
fn Echo() -> Element {
    let mut response = use_signal(|| String::new());

    rsx! {
        div {
            id: "echo",
            h4 { "ServerFn Echo" }
            input {
                placeholder: "Type here to echo...",
                oninput:  move |event| async move {
                    let data = echo_server(event.value()).await.unwrap();
                    response.set(data);
                },
            }

            if !response().is_empty() {
                p {
                    "Server echoed: "
                    i { "{response}" }
                }
            }
        }
    }
}

/// Echo the user input on the server.
#[server(EchoServer)]
async fn echo_server(input: String) -> Result<String, ServerFnError> {
    Ok(input)
}

/// VM Card Component
#[component]
fn VMCard(vm: VM) -> Element {
    let vm_id = vm.id.clone();
    let is_running = vm.status.is_running();
    
    rsx! {
        div { 
            class: "bg-white rounded-lg shadow-md hover:shadow-lg transition-shadow duration-200 p-6",
            // VM Header
            div { class: "flex items-center justify-between mb-4",
                h3 { class: "text-xl font-semibold text-gray-800", "{vm.name}" }
                div { 
                    class: format!("px-3 py-1 rounded-full text-sm font-medium {}",
                        match &vm.status {
                            VMStatus::Running { .. } => "bg-green-100 text-green-800",
                            VMStatus::Stopped => "bg-gray-100 text-gray-800",
                            VMStatus::Starting => "bg-yellow-100 text-yellow-800",
                            VMStatus::Stopping => "bg-orange-100 text-orange-800",
                            VMStatus::Error(_) => "bg-red-100 text-red-800",
                        }
                    ),
                    
                    "{vm.status.display_text()}"
                }
            }
            
            // VM Info
            div { class: "space-y-2 mb-4",
                div { class: "flex justify-between text-sm",
                    span { class: "text-gray-600 font-medium", "OS:" }
                    span { class: "text-gray-800", "{vm.os} {vm.version}" }
                }
                div { class: "flex justify-between text-sm",
                    span { class: "text-gray-600 font-medium", "CPU:" }
                    span { class: "text-gray-800", "{vm.cpu_cores} cores" }
                }
                div { class: "flex justify-between text-sm",
                    span { class: "text-gray-600 font-medium", "RAM:" }
                    span { class: "text-gray-800", "{vm.ram_mb / 1024}GB" }
                }
                div { class: "flex justify-between text-sm",
                    span { class: "text-gray-600 font-medium", "Disk:" }
                    span { class: "text-gray-800", "{vm.disk_size}" }
                }
            }
            
            // VM Actions
            div { class: "flex justify-end",
                if is_running {
                    button {
                        class: "bg-red-600 hover:bg-red-700 text-white font-medium py-2 px-4 rounded-md transition-colors duration-200 flex items-center space-x-2",
                        onclick: move |_| {
                            let id = vm_id.clone();
                            spawn(async move {
                                let _ = stop_vm(id).await;
                            });
                        },
                        span { "⏹" }
                        span { "Stop" }
                    }
                } else {
                    button {
                        class: "bg-green-600 hover:bg-green-700 text-white font-medium py-2 px-4 rounded-md transition-colors duration-200 flex items-center space-x-2",
                        onclick: move |_| {
                            let id = vm_id.clone();
                            spawn(async move {
                                let _ = start_vm(id).await;
                            });
                        },
                        span { "▶" }
                        span { "Start" }
                    }
                }
            }
        }
    }
}

/// Create VM Modal Component
#[component]
fn CreateVMModal(show: Signal<bool>, on_create: EventHandler<()>) -> Element {
    // State management
    let mut current_step = use_signal(|| 0);
    let mut popular_os_list = use_signal(Vec::<(String, Vec<String>)>::new);
    let mut all_os_list = use_signal(Vec::<(String, Vec<String>)>::new);
    let mut filtered_os_list = use_signal(Vec::<(String, Vec<String>)>::new);
    let mut search_query = use_signal(|| "".to_string());
    let mut selected_os = use_signal(|| "".to_string());
    let mut selected_version = use_signal(|| "".to_string());
    let mut selected_edition = use_signal(|| "".to_string());
    let mut available_editions = use_signal(Vec::<String>::new);
    let mut show_all_os = use_signal(|| false);
    
    // VM Configuration
    let mut vm_name = use_signal(|| "".to_string());
    let mut cpu_cores = use_signal(|| 2);
    let mut ram_gb = use_signal(|| 4);
    let mut disk_size_gb = use_signal(|| 20);
    let mut is_creating = use_signal(|| false);
    
    // Load OS lists
    use_effect(move || {
        spawn(async move {
            if let Ok(popular_list) = get_popular_os().await {
                popular_os_list.set(popular_list.clone());
                filtered_os_list.set(popular_list);
            }
            
            if let Ok(all_list) = get_available_os().await {
                all_os_list.set(all_list);
            }
        });
    });
    
    // Filter OS list based on search and show_all toggle
    use_effect(move || {
        let query = search_query().to_lowercase();
        let source_list = if show_all_os() { all_os_list() } else { popular_os_list() };
        
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
                if let Ok(editions) = get_os_editions(os, version).await {
                    available_editions.set(editions);
                }
            });
        } else {
            available_editions.set(vec![]);
        }
    });
    
    rsx! {
        div { 
            class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4",
            onclick: move |_| if !is_creating() { show.set(false) },
            
            div { 
                class: "bg-white rounded-2xl shadow-2xl w-full max-w-4xl max-h-[90vh] overflow-hidden",
                onclick: move |e| e.stop_propagation(),
                
                // Header with progress
                div { class: "bg-gradient-to-r from-blue-600 to-purple-600 text-white p-6",
                    div { class: "flex items-center justify-between mb-4",
                        h2 { class: "text-2xl font-bold", "Create New Virtual Machine" }
                        if !is_creating() {
                            button {
                                class: "text-white hover:text-gray-200 transition-colors",
                                onclick: move |_| show.set(false),
                                "✕"
                            }
                        }
                    }
                    
                    // Step indicators
                    div { class: "flex items-center space-x-4",
                        for (i, step_name) in ["OS Selection", "Configuration", "Review"].iter().enumerate() {
                            div { class: "flex items-center",
                                div { 
                                    class: format!("w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-colors {}",
                                        if i <= current_step() as usize { "bg-white text-blue-600" } else { "bg-blue-500 text-white" }
                                    ),
                                    "{i + 1}"
                                }
                                span { class: "ml-2 text-sm font-medium", "{step_name}" }
                                if i < 2 {
                                    div { class: "ml-4 w-8 h-px bg-blue-300" }
                                }
                            }
                        }
                    }
                }
                
                // Content
                div { class: "p-6 overflow-y-auto max-h-[60vh]",
                    match current_step() {
                        0 => rsx! { OSSelectionStep { 
                            search_query, 
                            show_all_os, 
                            filtered_os_list,
                            selected_os, 
                            selected_version, 
                            selected_edition,
                            available_editions
                        } },
                        1 => rsx! { ConfigurationStep { 
                            vm_name, 
                            cpu_cores, 
                            ram_gb, 
                            disk_size_gb 
                        } },
                        2 => rsx! { ReviewStep { 
                            vm_name: vm_name(),
                            selected_os: selected_os(),
                            selected_version: selected_version(),
                            selected_edition: selected_edition(),
                            cpu_cores: cpu_cores(),
                            ram_gb: ram_gb(),
                            disk_size_gb: disk_size_gb()
                        } },
                        _ => rsx! { div {} }
                    }
                }
                
                // Footer with navigation
                div { class: "bg-gray-50 px-6 py-4 flex justify-between items-center",
                    button {
                        class: format!("px-6 py-2 border border-gray-300 rounded-lg text-gray-700 hover:bg-gray-100 transition-colors {}",
                            if current_step() == 0 { "invisible" } else { "" }
                        ),
                        disabled: current_step() == 0 || is_creating(),
                        onclick: move |_| current_step.set(current_step() - 1),
                        "← Back"
                    }
                    
                    div { class: "flex space-x-3",
                        button {
                            class: "px-6 py-2 border border-gray-300 rounded-lg text-gray-700 hover:bg-gray-100 transition-colors",
                            disabled: is_creating(),
                            onclick: move |_| {
                                show.set(false);
                                current_step.set(0);
                            },
                            "Cancel"
                        }
                        
                        if current_step() < 2 {
                            button {
                                class: "px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:bg-gray-300 disabled:cursor-not-allowed",
                                disabled: {
                                    match current_step() {
                                        0 => selected_os().is_empty() || selected_version().is_empty(),
                                        1 => vm_name().is_empty() || cpu_cores() < 1 || ram_gb() < 1 || disk_size_gb() < 1,
                                        _ => false
                                    }
                                },
                                onclick: move |_| current_step.set(current_step() + 1),
                                "Next →"
                            }
                        } else {
                            button {
                                class: "px-6 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors disabled:bg-gray-400 disabled:cursor-not-allowed",
                                disabled: is_creating(),
                                onclick: move |_| {
                                    is_creating.set(true);
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
                                            ram: Some(ram_mb),
                                            disk_size: Some(disk_gb_str),
                                            cpu_cores: Some(cpu_cores()),
                                        };
                                        
                                        if let Ok(_) = create_vm(request).await {
                                            on_create.call(());
                                            show.set(false);
                                            current_step.set(0);
                                            is_creating.set(false);
                                        } else {
                                            is_creating.set(false);
                                        }
                                    });
                                },
                                if is_creating() { "🔄 Creating VM..." } else { "✓ Create VM" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// OS Selection Step Component
#[component]
fn OSSelectionStep(
    search_query: Signal<String>,
    show_all_os: Signal<bool>,
    filtered_os_list: Signal<Vec<(String, Vec<String>)>>,
    selected_os: Signal<String>,
    selected_version: Signal<String>,
    selected_edition: Signal<String>,
    available_editions: Signal<Vec<String>>,
) -> Element {
    rsx! {
        div { class: "space-y-6",
            // Search and toggle section
            div { class: "flex flex-col sm:flex-row gap-4 items-start sm:items-center",
                div { class: "flex-1",
                    input {
                        class: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                        placeholder: "Search operating systems...",
                        value: search_query(),
                        oninput: move |e| search_query.set(e.value()),
                    }
                }
                
                div { class: "flex items-center space-x-2",
                    label {
                        class: "flex items-center cursor-pointer",
                        input {
                            r#type: "checkbox",
                            class: "mr-2",
                            checked: show_all_os(),
                            onchange: move |e| show_all_os.set(e.value() == "true"),
                        }
                        span { class: "text-sm text-gray-700", "Show all OS ({}) instead of popular only" }
                    }
                }
            }
            
            // OS Grid
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 max-h-96 overflow-y-auto",
                for (os_name, versions) in filtered_os_list().iter() {
                    OSCard {
                        os_name: os_name.clone(),
                        versions: versions.clone(),
                        selected_os,
                        selected_version,
                    }
                }
            }
            
            // Version and Edition Selection
            if !selected_os().is_empty() {
                div { class: "mt-6 p-4 bg-gray-50 rounded-lg",
                    h4 { class: "text-lg font-semibold mb-3", "Configure {selected_os()}" }
                    
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                        // Version selection
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-2", "Version" }
                            select {
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                value: selected_version(),
                                onchange: move |e| {
                                    selected_version.set(e.value());
                                    selected_edition.set("".to_string()); // Reset edition
                                },
                                option { value: "", "Select version..." }
                                for version in filtered_os_list().iter()
                                    .find(|(name, _)| name == &selected_os())
                                    .map(|(_, versions)| versions)
                                    .unwrap_or(&vec![]) 
                                {
                                    option { value: "{version}", "{version}" }
                                }
                            }
                        }
                        
                        // Edition selection (if available)
                        if !available_editions().is_empty() {
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-2", "Edition (Optional)" }
                                select {
                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                                    value: selected_edition(),
                                    onchange: move |e| selected_edition.set(e.value()),
                                    option { value: "", "Default edition" }
                                    for edition in available_editions().iter() {
                                        option { value: "{edition}", "{edition}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// OS Card Component
#[component]
fn OSCard(
    os_name: String,
    versions: Vec<String>,
    selected_os: Signal<String>,
    selected_version: Signal<String>,
) -> Element {
    let is_selected = selected_os() == os_name;
    
    rsx! {
        div {
            class: format!("p-4 border-2 rounded-lg cursor-pointer transition-all duration-200 hover:shadow-md {}",
                if is_selected { "border-blue-500 bg-blue-50" } else { "border-gray-200 hover:border-gray-300" }
            ),
            onclick: move |_| {
                selected_os.set(os_name.clone());
                if !versions.is_empty() {
                    selected_version.set(versions[0].clone());
                }
            },
            
            div { class: "flex items-center justify-between",
                h3 { 
                    class: format!("font-semibold {}",
                        if is_selected { "text-blue-900" } else { "text-gray-900" }
                    ),
                    "{os_name}"
                }
                if is_selected {
                    span { class: "text-blue-500", "✓" }
                }
            }
            
            p { 
                class: format!("text-sm mt-1 {}",
                    if is_selected { "text-blue-700" } else { "text-gray-600" }
                ),
                "{versions.len()} version{if versions.len() == 1 { \"\" } else { \"s\" }} available"
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
                    class: "w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent",
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
            
            // Configuration Preview
            div { class: "mt-6 p-4 bg-gray-50 rounded-lg",
                h4 { class: "text-sm font-semibold text-gray-700 mb-2", "Configuration Summary" }
                div { class: "grid grid-cols-2 gap-4 text-sm",
                    div { class: "flex justify-between",
                        span { class: "text-gray-600", "CPU:" }
                        span { class: "font-medium", "{cpu_cores()} cores" }
                    }
                    div { class: "flex justify-between",
                        span { class: "text-gray-600", "RAM:" }
                        span { class: "font-medium", "{ram_gb()} GB" }
                    }
                    div { class: "flex justify-between",
                        span { class: "text-gray-600", "Disk:" }
                        span { class: "font-medium", "{disk_size_gb()} GB" }
                    }
                    div { class: "flex justify-between",
                        span { class: "text-gray-600", "Name:" }
                        span { class: "font-medium", 
                            if vm_name().is_empty() { "Auto-generated" } else { &vm_name() }
                        }
                    }
                }
            }
        }
    }
}

/// Review Step Component
#[component]
fn ReviewStep(
    vm_name: String,
    selected_os: String,
    selected_version: String,
    selected_edition: String,
    cpu_cores: i32,
    ram_gb: i32,
    disk_size_gb: i32,
) -> Element {
    rsx! {
        div { class: "space-y-6",
            div { class: "text-center mb-6",
                h3 { class: "text-xl font-bold text-gray-900 mb-2", "Review VM Configuration" }
                p { class: "text-gray-600", "Please review your VM settings before creation" }
            }
            
            // Configuration Review Cards
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                // OS Information
                div { class: "bg-gradient-to-br from-blue-50 to-blue-100 p-6 rounded-xl border border-blue-200",
                    h4 { class: "text-lg font-semibold text-blue-900 mb-4 flex items-center",
                        span { class: "mr-2", "💿" }
                        "Operating System"
                    }
                    div { class: "space-y-2",
                        div { class: "flex justify-between",
                            span { class: "text-blue-700", "OS:" }
                            span { class: "font-medium text-blue-900", "{selected_os}" }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-blue-700", "Version:" }
                            span { class: "font-medium text-blue-900", "{selected_version}" }
                        }
                        if !selected_edition.is_empty() {
                            div { class: "flex justify-between",
                                span { class: "text-blue-700", "Edition:" }
                                span { class: "font-medium text-blue-900", "{selected_edition}" }
                            }
                        }
                    }
                }
                
                // VM Configuration
                div { class: "bg-gradient-to-br from-green-50 to-green-100 p-6 rounded-xl border border-green-200",
                    h4 { class: "text-lg font-semibold text-green-900 mb-4 flex items-center",
                        span { class: "mr-2", "⚙️" }
                        "Hardware Configuration"
                    }
                    div { class: "space-y-2",
                        div { class: "flex justify-between",
                            span { class: "text-green-700", "Name:" }
                            span { class: "font-medium text-green-900", 
                                if vm_name.is_empty() { "Auto-generated" } else { &vm_name }
                            }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-green-700", "CPU Cores:" }
                            span { class: "font-medium text-green-900", "{cpu_cores}" }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-green-700", "RAM:" }
                            span { class: "font-medium text-green-900", "{ram_gb} GB" }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-green-700", "Disk Size:" }
                            span { class: "font-medium text-green-900", "{disk_size_gb} GB" }
                        }
                    }
                }
            }
            
            // Additional Information
            div { class: "bg-yellow-50 border border-yellow-200 p-4 rounded-lg",
                div { class: "flex items-start",
                    span { class: "text-yellow-600 mr-2 mt-1", "ℹ️" }
                    div {
                        h4 { class: "text-sm font-semibold text-yellow-800", "What happens next?" }
                        ul { class: "text-sm text-yellow-700 mt-2 space-y-1",
                            li { "• VM configuration file will be created" }
                            li { "• Operating system image will be downloaded (this may take some time)" }
                            li { "• VM will be ready to start once creation is complete" }
                            li { "• You can modify these settings later if needed" }
                        }
                    }
                }
            }
        }
    }
}