use core::fmt::Debug;
use core::time::Duration;

/// BackoffBuilder is utilized to construct a new backoff.
pub trait BackoffBuilder: Debug + Send + Sync + Unpin {
    /// The associated backoff returned by this builder.
    type Backoff: Backoff;

    /// Construct a new backoff using the builder.
    fn build(self) -> Self::Backoff;
}

/// Backoff is an [`Iterator`] that returns [`Duration`].
///
/// - `Some(Duration)` indicates the caller should `sleep(Duration)` and retry the request.
/// - `None` indicates the limits have been reached, and the caller should return the current error instead.
pub trait Backoff: Iterator<Item = Duration> + Send + Sync + Unpin {}
impl<T> Backoff for T where T: Iterator<Item = Duration> + Debug + Send + Sync + Unpin {}
