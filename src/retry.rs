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
    P: Backoff,
    T,
    E,
    Fut: Future<Output = std::result::Result<T, E>>,
    FutureFn: FnMut() -> Fut,
>
{
    fn retry(self, policy: P) -> Retry<P, T, E, Fut, FutureFn>;
}

impl<P, T, E, Fut, FutureFn> Retryable<P, T, E, Fut, FutureFn> for FutureFn
where
    P: Backoff,
    Fut: Future<Output = std::result::Result<T, E>>,
    FutureFn: FnMut() -> Fut,
{
    fn retry(self, policy: P) -> Retry<P, T, E, Fut, FutureFn> {
        Retry {
            backoff: policy,
            error_fn: |_: &E| true,
            future_fn: self,
            state: State::Idle,
        }
    }
}

#[pin_project]
pub struct Retry<
    P: Backoff,
    T,
    E,
    Fut: Future<Output = std::result::Result<T, E>>,
    FutureFn: FnMut() -> Fut,
> {
    backoff: P,
    error_fn: fn(&E) -> bool,
    future_fn: FutureFn,

    #[pin]
    state: State<T, E, Fut>,
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

impl<P, T, E, Fut, FutureFn> Future for Retry<P, T, E, Fut, FutureFn>
where
    P: Backoff,
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
                    Err(err) => match this.backoff.next() {
                        None => return Poll::Ready(Err(err)),
                        Some(dur) => {
                            this.state
                                .set(State::Sleeping(Box::pin(tokio::time::sleep(dur))));
                            continue;
                        }
                    },
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
}
