#[cfg(feature = "web-server")]
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
#[cfg(feature = "web-server")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "web-server")]
use std::sync::Arc;
#[cfg(feature = "web-server")]
use tokio::sync::RwLock;
#[cfg(feature = "web-server")]
use tower_http::cors::CorsLayer;

#[cfg(feature = "web-server")]
use crate::models::vm::{VM, VMId};
#[cfg(feature = "web-server")]
use crate::services::{VMManager, VMDiscovery, DiscoveryEvent};

#[cfg(feature = "web-server")]
#[derive(Clone)]
pub struct AppState {
    pub vm_manager: Arc<VMManager>,
    pub vm_directories: Vec<std::path::PathBuf>,
}

#[cfg(feature = "web-server")]
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[cfg(feature = "web-server")]
#[derive(Deserialize)]
pub struct StartVMRequest {
    pub vm_id: VMId,
}

#[cfg(feature = "web-server")]
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/vms", get(list_vms))
        .route("/api/vms/:id/start", post(start_vm))
        .route("/api/vms/:id/stop", post(stop_vm))
        .route("/api/vms/:id/status", get(get_vm_status))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[cfg(feature = "web-server")]
async fn list_vms(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<VM>>>, StatusCode> {
    let mut all_vms = Vec::new();
    
    for dir in &state.vm_directories {
        let (event_tx, _) = tokio::sync::mpsc::unbounded_channel::<DiscoveryEvent>();
        let mut discovery = VMDiscovery::with_vm_manager(event_tx, state.vm_manager.clone());
        
        match discovery.scan_directory(dir).await {
            Ok(vms) => all_vms.extend(vms),
            Err(e) => {
                return Ok(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to scan directory {}: {}", dir.display(), e)),
                }));
            }
        }
    }
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(all_vms),
        error: None,
    }))
}

#[cfg(feature = "web-server")]
async fn start_vm(
    axum::extract::Path(vm_id): axum::extract::Path<VMId>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    // First find the VM
    let mut target_vm = None;
    for dir in &state.vm_directories {
        let (event_tx, _) = tokio::sync::mpsc::unbounded_channel::<DiscoveryEvent>();
        let mut discovery = VMDiscovery::with_vm_manager(event_tx, state.vm_manager.clone());
        
        if let Ok(vms) = discovery.scan_directory(dir).await {
            if let Some(vm) = vms.into_iter().find(|v| v.id == vm_id) {
                target_vm = Some(vm);
                break;
            }
        }
    }
    
    match target_vm {
        Some(vm) => {
            match state.vm_manager.start_vm(&vm).await {
                Ok(_) => Ok(Json(ApiResponse {
                    success: true,
                    data: Some(()),
                    error: None,
                })),
                Err(e) => Ok(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })),
            }
        }
        None => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("VM with ID {:?} not found", vm_id)),
        })),
    }
}

#[cfg(feature = "web-server")]
async fn stop_vm(
    axum::extract::Path(vm_id): axum::extract::Path<VMId>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.vm_manager.stop_vm(&vm_id).await {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            data: Some(()),
            error: None,
        })),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

#[cfg(feature = "web-server")]
async fn get_vm_status(
    axum::extract::Path(vm_id): axum::extract::Path<VMId>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<crate::models::vm::VMStatus>>, StatusCode> {
    let status = state.vm_manager.get_vm_status(&vm_id).await;
    Ok(Json(ApiResponse {
        success: true,
        data: Some(status),
        error: None,
    }))
}

#[cfg(feature = "web-server")]
pub async fn start_web_server(vm_manager: Arc<VMManager>, vm_directories: Vec<std::path::PathBuf>) -> anyhow::Result<()> {
    let state = AppState { vm_manager, vm_directories };
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("Web server starting on http://127.0.0.1:3000");
    
    axum::serve(listener, app).await?;
    Ok(())
}