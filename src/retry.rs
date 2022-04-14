use crate::{ExponentialBackoff, Policy};
use futures::ready;
use pin_project::pin_project;
use std::env::Args;
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

trait Retryable<T, E, Fut: Future<Output = std::result::Result<T, E>>, FutureFn: FnMut() -> Fut> {
    fn retry(self) -> Retry<T, E, Fut, FutureFn>;
}

impl<T, E, Fut, FutureFn> Retryable<T, E, Fut, FutureFn> for FutureFn
where
    Fut: Future<Output = std::result::Result<T, E>>,
    FutureFn: FnMut() -> Fut,
{
    fn retry(self) -> Retry<T, E, Fut, FutureFn> {
        Retry {
            backoff: ExponentialBackoff::default(),
            error_fn: |_: &E| true,
            future_fn: self,
            state: State::Idle,
        }
    }
}

#[pin_project]
struct Retry<T, E, Fut: Future<Output = std::result::Result<T, E>>, FutureFn: FnMut() -> Fut> {
    backoff: ExponentialBackoff,
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
    Sleeping(#[pin] tokio::time::Sleep),
}

impl<T, E, Fut> Default for State<T, E, Fut>
where
    Fut: Future<Output = std::result::Result<T, E>>,
{
    fn default() -> Self {
        State::Idle
    }
}

impl<T, E, Fut, FutureFn> Future for Retry<T, E, Fut, FutureFn>
where
    Fut: Future<Output = std::result::Result<T, E>>,
    FutureFn: FnMut() -> Fut,
{
    type Output = std::result::Result<T, E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
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
                            this.state.set(State::Sleeping(tokio::time::sleep(dur)));
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

pub async fn retry<B, T, E, FUT, FF, FE, FS, FSF>(
    mut backoff: B,
    mut sleep_fn: FS,
    mut error_fn: FE,
    mut future_fn: FF,
) -> std::result::Result<T, E>
where
    B: Policy,
    FF: FnMut() -> FUT,
    FE: FnMut(&E) -> bool,
    FS: FnMut(Duration) -> FSF,
    FUT: Future<Output = std::result::Result<T, E>>,
    FSF: Future<Output = ()>,
{
    loop {
        match (future_fn)().await {
            Ok(v) => return Ok(v),
            Err(err) => {
                if !(error_fn)(&err) {
                    return Err(err);
                }

                match backoff.next() {
                    None => return Err(err),
                    Some(dur) => {
                        (sleep_fn)(dur).await;
                        continue;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        .retry()
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
        let x = test_query.retry().await?;

        println!("got: {:?}", x);

        Ok(())
    }
}
