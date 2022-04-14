use std::time::Duration;

/// Backoff is an [`Iterator`] that returns [`Duration`].
///
/// - `Some(Duration)` means caller need to `sleep(Duration)` and retry the same request
/// - `None` means we have reaching the limits, caller needs to return current error instead.
pub trait Backoff: Iterator<Item = Duration> + Clone {}
impl<T> Backoff for T where T: Iterator<Item = Duration> + Clone {}
