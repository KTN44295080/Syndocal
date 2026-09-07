use crate::native_thumbnail_work::ThumbnailJob;
use std::sync::atomic::AtomicBool;

/// A dropped async waiter requests cancellation but never releases the worker's slot.
/// Blocking decode/I/O must return before that slot can admit another request.
pub(crate) async fn run<T, F>(job: ThumbnailJob, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&AtomicBool) -> Result<T, String> + Send + 'static,
{
    let request_guard = job.request_guard();
    let result = tauri::async_runtime::spawn_blocking(move || job.execute(operation))
        .await
        .map_err(|error| format!("Thumbnail worker failed: {error}"))?;
    drop(request_guard);
    result
}

#[cfg(test)]
#[path = "native_thumbnail_dispatch_tests.rs"]
mod tests;
