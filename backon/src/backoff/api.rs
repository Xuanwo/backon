use core::time::Duration;

/// Backoff is an [`Iterator`] that returns [`Duration`].
///
/// - `Some(Duration)` indicates the caller should `sleep(Duration)` and retry the request.
/// - `None` indicates the limits have been reached, and the caller should return the current error instead.
pub trait Backoff: Iterator<Item = Duration> + Send + Sync + Unpin {}
impl<T> Backoff for T where T: Iterator<Item = Duration> + Send + Sync + Unpin {}

/// BackoffBuilder is utilized to construct a new backoff.
pub trait BackoffBuilder: Send + Sync + Unpin {
    /// The associated backoff returned by this builder.
    type Backoff: Backoff;

    /// Construct a new backoff using the builder.
    fn build(self) -> Self::Backoff;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConstantBuilder, ExponentialBuilder, FibonacciBuilder};

    fn test_fn_builder(b: impl BackoffBuilder) {
        let _ = b.build();
    }

    #[test]
    fn test_backoff_builder() {
        // We should only test with proper builders

        // Just for test if user can keep using &XxxBuilder.
        #[allow(clippy::needless_borrows_for_generic_args)]
        {
            test_fn_builder(&ConstantBuilder::default());
            test_fn_builder(&FibonacciBuilder::default());
            test_fn_builder(&ExponentialBuilder::default());
        }
    }
}
