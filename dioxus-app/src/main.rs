use dioxus::prelude::*;

mod models;
mod server_functions;

use crate::models::{VM, VMStatus, CreateVMRequest};
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
    let mut os_list = use_signal(Vec::<(String, Vec<String>)>::new);
    let mut selected_os = use_signal(|| "".to_string());
    let mut selected_version = use_signal(|| "".to_string());
    
    // Load available OS list
    use_effect(move || {
        spawn(async move {
            if let Ok(list) = get_available_os().await {
                os_list.set(list);
            }
        });
    });
    
    rsx! {
        div { 
            class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            onclick: move |_| show.set(false),
            
            div { 
                class: "bg-white rounded-lg shadow-xl p-6 w-full max-w-md",
                onclick: move |e| e.stop_propagation(),
                
                h2 { class: "text-2xl font-bold text-gray-800 mb-6", "Create New VM" }
                
                // OS Selection
                div { class: "mb-4",
                    label { class: "block text-sm font-medium text-gray-700 mb-2", "Operating System" }
                    select {
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                        onchange: move |e| selected_os.set(e.value()),
                        option { value: "", "Select OS..." }
                        for (os, _) in os_list().iter() {
                            option { value: "{os}", "{os}" }
                        }
                    }
                }
                
                // Version Selection
                if !selected_os().is_empty() {
                    div { class: "mb-6",
                        label { class: "block text-sm font-medium text-gray-700 mb-2", "Version" }
                        select {
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                            onchange: move |e| selected_version.set(e.value()),
                            option { value: "", "Select Version..." }
                            for (os, versions) in os_list().iter() {
                                if os == &selected_os() {
                                    for version in versions {
                                        option { value: "{version}", "{version}" }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Actions
                div { class: "flex justify-end space-x-3 mt-6",
                    button {
                        class: "px-4 py-2 border border-gray-300 rounded-md text-gray-700 hover:bg-gray-50 transition-colors",
                        onclick: move |_| show.set(false),
                        "Cancel"
                    }
                    button {
                        class: "px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors disabled:bg-gray-300 disabled:cursor-not-allowed",
                        disabled: selected_os().is_empty() || selected_version().is_empty(),
                        onclick: move |_| {
                            let os = selected_os();
                            let version = selected_version();
                            spawn(async move {
                                let request = CreateVMRequest {
                                    os: os.clone(),
                                    version: version.clone(),
                                    edition: None,
                                };
                                if let Ok(_) = create_vm(request).await {
                                    on_create.call(());
                                    show.set(false);
                                }
                            });
                        },
                        "Create VM"
                    }
                }
            }
        }
    }
}
