use instant::Duration;

/// Cross-platform async sleep function
pub async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
    
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(duration).await;
}

/// Task handle for cross-platform compatibility
#[cfg(not(target_arch = "wasm32"))]
pub type TaskHandle<T> = tokio::task::JoinHandle<T>;

#[cfg(target_arch = "wasm32")]
pub type TaskHandle = ();

/// Cross-platform task spawning
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_task<F>(future: F) -> TaskHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_task<F>(future: F) -> TaskHandle
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
    ()
}