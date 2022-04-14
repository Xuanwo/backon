use futures::ready;
use pin_project::pin_project;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Backoff;

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
                    // this.state = State::Polling(fut);
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
    use super::*;
    use crate::exponential::ExponentialBackoff;

    #[tokio::test]
    async fn test_retry() -> anyhow::Result<()> {
        let x = {
            || async {
                let x = reqwest::get("https://www.rust-lang.org")
                    .await?
                    .text()
                    .await?;

                Err(anyhow::anyhow!(x))
            }
        }
        .retry(ExponentialBackoff::default())
        .await?;

        println!("got: {:?}", x);

        Ok(())
    }

    async fn test_query() -> anyhow::Result<()> {
        let x = reqwest::get("https://www.rust-lang.org")
            .await?
            .text()
            .await?;

        Err(anyhow::anyhow!(x))
    }

    #[tokio::test]
    async fn test_retry_x() -> anyhow::Result<()> {
        let x = test_query.retry(ExponentialBackoff::default()).await?;

        println!("got: {:?}", x);

        Ok(())
    }
}
