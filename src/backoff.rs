use std::fmt::Debug;
use std::time::Duration;

/// BackoffBuilder is used to build a new backoff.
pub trait BackoffBuilder: Clone + Debug + Send + Sync + Unpin {
    /// The associated backoff that returned by this builder.
    type Backoff: Backoff;

    /// Builder a new backoff via builder.
    fn build(&self) -> Self::Backoff;
}

/// Backoff is an [`Iterator`] that returns [`Duration`].
///
/// - `Some(Duration)` means caller need to `sleep(Duration)` and retry the same request
/// - `None` means we have reaching the limits, caller needs to return current error instead.
pub trait Backoff: Iterator<Item = Duration> + Send + Sync + Unpin {}
impl<T> Backoff for T where T: Iterator<Item = Duration> + Debug + Send + Sync + Unpin {}
