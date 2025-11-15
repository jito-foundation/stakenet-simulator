use std::future::Future;
use std::time::Duration;
use tracing::warn;

/// Retry a future with exponential backoff
///
/// # Arguments
/// * `max_retries` - Maximum number of retry attempts
/// * `initial_delay_ms` - Initial delay in milliseconds before first retry
/// * `max_delay_ms` - Maximum delay in milliseconds between retries
/// * `operation` - The async operation to retry
pub async fn retry_with_exponential_backoff<F, Fut, T, E>(
    max_retries: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    let mut delay_ms = initial_delay_ms;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt > max_retries {
                    return Err(e);
                }

                warn!(
                    "Attempt {}/{} failed: {}. Retrying in {}ms...",
                    attempt, max_retries, e, delay_ms
                );

                tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                // Exponential backoff with cap
                delay_ms = (delay_ms * 2).min(max_delay_ms);
            }
        }
    }
}
