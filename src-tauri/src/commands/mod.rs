//! Tauri commands: thin glue between the Svelte UI and the use cases in `vuretro-core`.
//! No rules live here; each command unwraps its arguments, calls `App` and shapes the reply.

pub mod config;
pub mod cores;
pub mod covers;
pub mod launch;
pub mod library;

/// Run a blocking use case off the UI thread.
pub(crate) async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())
}
