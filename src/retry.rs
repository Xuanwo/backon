use crate::Policy;
use futures::{pin_mut, ready, TryFuture};
use pin_project::pin_project;
use std::error::Error;
use std::future::Future;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait Retryable<B: Policy, F: Fn(&Self::Error) -> bool>: TryFuture + Sized {
    fn retry(self, backoff: B, handle: F) -> Retry<Self, B, F> {
        Retry {
            inner: self,
            backoff,
            handle,
            sleeper: None,
        }
    }
}

impl<T, B, F> Retryable<B, F> for T
where
    T: TryFuture,
    B: Policy,
    F: Fn(&T::Error) -> bool,
{
}

#[pin_project]
pub struct Retry<T: TryFuture, B: Policy, F: Fn(&T::Error) -> bool> {
    #[pin]
    inner: T,
    backoff: B,
    handle: F,
    sleeper: Option<Pin<Box<tokio::time::Sleep>>>,
}

impl<T, B, F> Future for Retry<T, B, F>
where
    T: TryFuture,
    F: Fn(&T::Error) -> bool,
    B: Policy,
{
    type Output = Result<T::Ok, T::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        if let Some(sleeper) = this.sleeper.as_mut() {
            ready!(sleeper.as_mut().poll(cx));
            *this.sleeper = None;
        }

        match ready!(this.inner.try_poll(cx)) {
            Ok(v) => Poll::Ready(Self::Output::Ok(v)),
            Err(err) => {
                if !(this.handle)(&err) {
                    return Poll::Ready(Self::Output::Err(err));
                }

                match this.backoff.next() {
                    None => Poll::Ready(Self::Output::Err(err)),
                    Some(v) => {
                        *this.sleeper = Some(Box::pin(tokio::time::sleep(v)));

                        if let Some(sleeper) = this.sleeper.as_mut() {
                            ready!(sleeper.as_mut().poll(cx));
                            *this.sleeper = None;
                        }
                        Poll::Pending
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Retryable;
    use super::*;
    use crate::ExponentialBackoff;
    use anyhow::{anyhow, Result};
    use futures::TryFuture;

    async fn test_fn() -> Result<()> {
        Err(anyhow!("error"))
    }

    #[tokio::test]
    async fn test_retry() {
        let backoff = ExponentialBackoff::default().with_max_times(1);
        test_fn().retry(backoff, |_| true).await;
    }
}
