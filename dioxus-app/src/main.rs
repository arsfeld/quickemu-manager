mod components;

#[cfg(feature = "web")]
mod api;

#[cfg(any(feature = "desktop", feature = "server"))]
mod services;

use components::app::App;

fn main() {
    #[cfg(feature = "desktop")]
    {
        dioxus::LaunchBuilder::desktop()
            .with_cfg(dioxus::desktop::Config::new()
                .with_window(dioxus::desktop::WindowBuilder::new()
                    .with_title("Quickemu Manager")
                    .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))))
            .launch(App);
    }

    #[cfg(feature = "web")]
    {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::default());
        dioxus::launch(App);
    }

    #[cfg(feature = "server")]
    {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                launch_server().await;
            });
    }
}

#[cfg(feature = "server")]
async fn launch_server() {
    use axum::{
        routing::{get, post},
        Router,
    };
    use tower_http::cors::CorsLayer;
    
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/vms", get(api_list_vms))
        .route("/api/vms/:id/start", post(api_start_vm))
        .route("/api/vms/:id/stop", post(api_stop_vm))
        .route("/api/vms/:id/restart", post(api_restart_vm))
        .route("/api/vms/:id", get(api_get_vm))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    
    println!("Server running at http://0.0.0.0:8080");
    
    axum::serve(listener, app).await.unwrap();
}

#[cfg(feature = "server")]
async fn serve_index() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../../index.html"))
}

#[cfg(feature = "server")]
async fn api_list_vms() -> axum::Json<Vec<crate::services::VM>> {
    use crate::services::vm_manager::VMManager;
    
    let manager = VMManager::new();
    let vms = manager.list_vms().await.unwrap_or_default();
    axum::Json(vms)
}

#[cfg(feature = "server")]
async fn api_start_vm(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> axum::response::Result<axum::Json<&'static str>> {
    use crate::services::vm_manager::VMManager;
    
    let manager = VMManager::new();
    match manager.start_vm(&id).await {
        Ok(_) => Ok(axum::Json("VM started")),
        Err(e) => Err(format!("Failed to start VM: {}", e).into()),
    }
}

#[cfg(feature = "server")]
async fn api_stop_vm(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> axum::response::Result<axum::Json<&'static str>> {
    use crate::services::vm_manager::VMManager;
    
    let manager = VMManager::new();
    match manager.stop_vm(&id).await {
        Ok(_) => Ok(axum::Json("VM stopped")),
        Err(e) => Err(format!("Failed to stop VM: {}", e).into()),
    }
}

#[cfg(feature = "server")]
async fn api_restart_vm(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> axum::response::Result<axum::Json<&'static str>> {
    use crate::services::vm_manager::VMManager;
    
    let manager = VMManager::new();
    match manager.restart_vm(&id).await {
        Ok(_) => Ok(axum::Json("VM restarted")),
        Err(e) => Err(format!("Failed to restart VM: {}", e).into()),
    }
}

#[cfg(feature = "server")]
async fn api_get_vm(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> axum::response::Result<axum::Json<crate::services::VM>> {
    use crate::services::vm_manager::VMManager;
    
    let manager = VMManager::new();
    match manager.get_vm(&id).await {
        Ok(vm) => Ok(axum::Json(vm)),
        Err(e) => Err(format!("VM not found: {}", e).into()),
    }
}