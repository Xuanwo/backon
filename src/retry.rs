use futures::ready;
use pin_project::pin_project;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Backoff;

/// Retryable will add retry support for functions that produces a futures with results.
///
/// That means all types that implement `FnMut() -> impl Future<Output = std::result::Result<T, E>>`
/// will be able to use `retry`.
///
/// For example:
///
/// - Functions without extra args:
///
/// ```ignore
/// async fn fetch() -> Result<String> {
///     Ok(reqwest::get("https://www.rust-lang.org").await?.text().await?)
/// }
/// ```
///
/// - Closures
///
/// ```ignore
/// || async {
///     let x = reqwest::get("https://www.rust-lang.org")
///         .await?
///         .text()
///         .await?;
///
///     Err(anyhow::anyhow!(x))
/// }
/// ```
///
/// # Example
///
/// ```no_run
/// use backon::Retryable;
/// use backon::ExponentialBackoff;
/// use anyhow::Result;
///
/// async fn fetch() -> Result<String> {
///     Ok(reqwest::get("https://www.rust-lang.org").await?.text().await?)
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let content = fetch.retry(ExponentialBackoff::default()).await?;
///     println!("fetch succeeded: {}", content);
///
///     Ok(())
/// }
/// ```
pub trait Retryable<
    B: Backoff,
    T,
    E,
    Fut: Future<Output = std::result::Result<T, E>>,
    FutureFn: FnMut() -> Fut,
>
{
    fn retry(self, backoff: B) -> Retry<B, T, E, Fut, FutureFn>;
}

impl<B, T, E, Fut, FutureFn> Retryable<B, T, E, Fut, FutureFn> for FutureFn
where
    B: Backoff,
    Fut: Future<Output = std::result::Result<T, E>>,
    FutureFn: FnMut() -> Fut,
{
    fn retry(self, backoff: B) -> Retry<B, T, E, Fut, FutureFn> {
        Retry::new(self, backoff)
    }
}

#[pin_project]
pub struct Retry<
    B: Backoff,
    T,
    E,
    Fut: Future<Output = std::result::Result<T, E>>,
    FutureFn: FnMut() -> Fut,
> {
    backoff: B,
    error_fn: fn(&E) -> bool,
    future_fn: FutureFn,

    #[pin]
    state: State<T, E, Fut>,
}

impl<B, T, E, Fut, FutureFn> Retry<B, T, E, Fut, FutureFn>
where
    B: Backoff,
    Fut: Future<Output = std::result::Result<T, E>>,
    FutureFn: FnMut() -> Fut,
{
    /// Create a new retry.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use backon::Retryable;
    /// use backon::Retry;
    /// use backon::ExponentialBackoff;
    /// use anyhow::Result;
    ///
    /// async fn fetch() -> Result<String> {
    ///     Ok(reqwest::get("https://www.rust-lang.org").await?.text().await?)
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let content = Retry::new(fetch, ExponentialBackoff::default()).await?;
    ///     println!("fetch succeeded: {}", content);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn new(future_fn: FutureFn, backoff: B) -> Self {
        Retry {
            backoff,
            error_fn: |_: &E| true,
            future_fn,
            state: State::Idle,
        }
    }

    /// Set error_fn of retry
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use backon::Retry;
    /// use backon::ExponentialBackoff;
    /// use anyhow::Result;
    ///
    /// async fn fetch() -> Result<String> {
    ///     Ok(reqwest::get("https://www.rust-lang.org").await?.text().await?)
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let retry = Retry::new(fetch, ExponentialBackoff::default())
    ///             .with_error_fn(|e| e.to_string() == "EOF");
    ///     let content = retry.await?;
    ///     println!("fetch succeeded: {}", content);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn with_error_fn(mut self, error_fn: fn(&E) -> bool) -> Self {
        self.error_fn = error_fn;
        self
    }
}

/// State maintains internal state of retry.
///
/// # Notes
///
/// `tokio::time::Sleep` is a very struct that occupy 640B, so we wrap it
/// into a `Pin<Box<_>>` to avoid this enum too large.
#[pin_project(project = StateProject)]
enum State<T, E, Fut: Future<Output = std::result::Result<T, E>>> {
    Idle,
    Polling(#[pin] Fut),
    // TODO: we need to support other sleeper
    Sleeping(#[pin] Pin<Box<tokio::time::Sleep>>),
}

impl<T, E, Fut> Default for State<T, E, Fut>
where
    Fut: Future<Output = std::result::Result<T, E>>,
{
    fn default() -> Self {
        State::Idle
    }
}

impl<B, T, E, Fut, FutureFn> Future for Retry<B, T, E, Fut, FutureFn>
where
    B: Backoff,
    Fut: Future<Output = std::result::Result<T, E>>,
    FutureFn: FnMut() -> Fut,
{
    type Output = std::result::Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            let state = this.state.as_mut().project();
            match state {
                StateProject::Idle => {
                    let fut = (this.future_fn)();
                    this.state.set(State::Polling(fut));
                    continue;
                }
                StateProject::Polling(fut) => match ready!(fut.poll(cx)) {
                    Ok(v) => return Poll::Ready(Ok(v)),
                    Err(err) => {
                        // If input error is not retryable, return error directly.
                        if !(this.error_fn)(&err) {
                            return Poll::Ready(Err(err));
                        }
                        match this.backoff.next() {
                            None => return Poll::Ready(Err(err)),
                            Some(dur) => {
                                this.state
                                    .set(State::Sleeping(Box::pin(tokio::time::sleep(dur))));
                                continue;
                            }
                        }
                    }
                },
                StateProject::Sleeping(sl) => {
                    ready!(sl.poll(cx));
                    this.state.set(State::Idle);
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use tokio::sync::Mutex;

    use super::*;
    use crate::exponential::ExponentialBackoff;

    async fn always_error() -> anyhow::Result<()> {
        Err(anyhow::anyhow!("test_query meets error"))
    }

    #[tokio::test]
    async fn test_retry() -> anyhow::Result<()> {
        let result = always_error
            .retry(ExponentialBackoff::default().with_min_delay(Duration::from_millis(1)))
            .await;

        assert!(result.is_err());
        assert_eq!("test_query meets error", result.unwrap_err().to_string());
        Ok(())
    }

    #[tokio::test]
    async fn test_retry_with_not_retryable_error() -> anyhow::Result<()> {
        let error_times = Mutex::new(0);

        let f = || async {
            let mut x = error_times.lock().await;
            *x += 1;
            Err::<(), anyhow::Error>(anyhow::anyhow!("not retryable"))
        };

        let backoff = ExponentialBackoff::default().with_min_delay(Duration::from_millis(1));
        let result = f
            .retry(backoff)
            // Only retry If error message is `retryable`
            .with_error_fn(|e| e.to_string() == "retryable")
            .await;

        assert!(result.is_err());
        assert_eq!("not retryable", result.unwrap_err().to_string());
        // `f` always returns error "not retryable", so it should be executed
        // only once.
        assert_eq!(*error_times.lock().await, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_retry_with_retryable_error() -> anyhow::Result<()> {
        let error_times = Mutex::new(0);

        let f = || async {
            let mut x = error_times.lock().await;
            *x += 1;
            Err::<(), anyhow::Error>(anyhow::anyhow!("retryable"))
        };

        let backoff = ExponentialBackoff::default().with_min_delay(Duration::from_millis(1));
        let result = f
            .retry(backoff)
            // Only retry If error message is `retryable`
            .with_error_fn(|e| e.to_string() == "retryable")
            .await;

        assert!(result.is_err());
        assert_eq!("retryable", result.unwrap_err().to_string());
        // `f` always returns error "retryable", so it should be executed
        // 4 times (retry 3 times).
        assert_eq!(*error_times.lock().await, 4);
        Ok(())
    }
}
