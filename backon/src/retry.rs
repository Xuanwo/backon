use core::future::Future;
use core::pin::Pin;
use core::task::ready;
use core::task::Context;
use core::task::Poll;
use core::time::Duration;

use crate::backoff::BackoffBuilder;
use crate::sleep::MaybeSleeper;
use crate::Backoff;
use crate::DefaultSleeper;
use crate::Sleeper;

/// Retryable will add retry support for functions that produce futures with results.
///
/// This means all types that implement `FnMut() -> impl Future<Output = Result<T, E>>`
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
pub trait Retryable<
    B: BackoffBuilder,
    T,
    E,
    Fut: Future<Output = Result<T, E>>,
    FutureFn: FnMut() -> Fut,
>
{
    /// Generate a new retry
    fn retry(self, builder: B) -> Retry<B::Backoff, T, E, Fut, FutureFn>;
}

impl<B, T, E, Fut, FutureFn> Retryable<B, T, E, Fut, FutureFn> for FutureFn
where
    B: BackoffBuilder,
    Fut: Future<Output = Result<T, E>>,
    FutureFn: FnMut() -> Fut,
{
    fn retry(self, builder: B) -> Retry<B::Backoff, T, E, Fut, FutureFn> {
        Retry::new(self, builder.build())
    }
}

/// Struct generated by [`Retryable`].
pub struct Retry<
    B: Backoff,
    T,
    E,
    Fut: Future<Output = Result<T, E>>,
    FutureFn: FnMut() -> Fut,
    SF: MaybeSleeper = DefaultSleeper,
    RF = fn(&E) -> bool,
    NF = fn(&E, Duration),
> {
    backoff: B,
    future_fn: FutureFn,

    retryable_fn: RF,
    notify_fn: NF,
    sleep_fn: SF,

    state: State<T, E, Fut, SF::Sleep>,
}

impl<B, T, E, Fut, FutureFn> Retry<B, T, E, Fut, FutureFn>
where
    B: Backoff,
    Fut: Future<Output = Result<T, E>>,
    FutureFn: FnMut() -> Fut,
{
    /// Initiate a new retry.
    fn new(future_fn: FutureFn, backoff: B) -> Self {
        Retry {
            backoff,
            future_fn,

            retryable_fn: |_: &E| true,
            notify_fn: |_: &E, _: Duration| {},
            sleep_fn: DefaultSleeper::default(),

            state: State::Idle,
        }
    }
}

impl<B, T, E, Fut, FutureFn, SF, RF, NF> Retry<B, T, E, Fut, FutureFn, SF, RF, NF>
where
    B: Backoff,
    Fut: Future<Output = Result<T, E>>,
    FutureFn: FnMut() -> Fut,
    SF: MaybeSleeper,
    RF: FnMut(&E) -> bool,
    NF: FnMut(&E, Duration),
{
    /// Set the sleeper for retrying.
    ///
    /// The sleeper should implement the [`Sleeper`] trait. The simplest way is to use a closure that returns a `Future<Output=()>`.
    ///
    /// If not specified, we use the [`DefaultSleeper`].
    ///
    /// ```no_run
    /// use anyhow::Result;
    /// use backon::ExponentialBuilder;
    /// use backon::Retryable;
    /// use std::future::ready;
    ///
    /// async fn fetch() -> Result<String> {
    ///     Ok(reqwest::get("https://www.rust-lang.org")
    ///         .await?
    ///         .text()
    ///         .await?)
    /// }
    ///
    /// #[tokio::main(flavor = "current_thread")]
    /// async fn main() -> Result<()> {
    ///     let content = fetch
    ///         .retry(ExponentialBuilder::default())
    ///         .sleep(|_| ready(()))
    ///         .await?;
    ///     println!("fetch succeeded: {}", content);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn sleep<SN: Sleeper>(self, sleep_fn: SN) -> Retry<B, T, E, Fut, FutureFn, SN, RF, NF> {
        Retry {
            backoff: self.backoff,
            retryable_fn: self.retryable_fn,
            notify_fn: self.notify_fn,
            future_fn: self.future_fn,
            sleep_fn,
            state: State::Idle,
        }
    }

    /// Set the conditions for retrying.
    ///
    /// If not specified, all errors are considered retryable.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use anyhow::Result;
    /// use backon::ExponentialBuilder;
    /// use backon::Retryable;
    ///
    /// async fn fetch() -> Result<String> {
    ///     Ok(reqwest::get("https://www.rust-lang.org")
    ///         .await?
    ///         .text()
    ///         .await?)
    /// }
    ///
    /// #[tokio::main(flavor = "current_thread")]
    /// async fn main() -> Result<()> {
    ///     let content = fetch
    ///         .retry(ExponentialBuilder::default())
    ///         .when(|e| e.to_string() == "EOF")
    ///         .await?;
    ///     println!("fetch succeeded: {}", content);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn when<RN: FnMut(&E) -> bool>(
        self,
        retryable: RN,
    ) -> Retry<B, T, E, Fut, FutureFn, SF, RN, NF> {
        Retry {
            backoff: self.backoff,
            retryable_fn: retryable,
            notify_fn: self.notify_fn,
            future_fn: self.future_fn,
            sleep_fn: self.sleep_fn,
            state: self.state,
        }
    }

    /// Set to notify for all retry attempts.
    ///
    /// When a retry happens, the input function will be invoked with the error and the sleep duration before pausing.
    ///
    /// If not specified, this operation does nothing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use core::time::Duration;
    ///
    /// use anyhow::Result;
    /// use backon::ExponentialBuilder;
    /// use backon::Retryable;
    ///
    /// async fn fetch() -> Result<String> {
    ///     Ok(reqwest::get("https://www.rust-lang.org")
    ///         .await?
    ///         .text()
    ///         .await?)
    /// }
    ///
    /// #[tokio::main(flavor = "current_thread")]
    /// async fn main() -> Result<()> {
    ///     let content = fetch
    ///         .retry(ExponentialBuilder::default())
    ///         .notify(|err: &anyhow::Error, dur: Duration| {
    ///             println!("retrying error {:?} with sleeping {:?}", err, dur);
    ///         })
    ///         .await?;
    ///     println!("fetch succeeded: {}", content);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn notify<NN: FnMut(&E, Duration)>(
        self,
        notify: NN,
    ) -> Retry<B, T, E, Fut, FutureFn, SF, RF, NN> {
        Retry {
            backoff: self.backoff,
            retryable_fn: self.retryable_fn,
            notify_fn: notify,
            sleep_fn: self.sleep_fn,
            future_fn: self.future_fn,
            state: self.state,
        }
    }
}

/// State maintains internal state of retry.
#[derive(Default)]
enum State<T, E, Fut: Future<Output = Result<T, E>>, SleepFut: Future<Output = ()>> {
    #[default]
    Idle,
    Polling(Fut),
    Sleeping(SleepFut),
}

impl<B, T, E, Fut, FutureFn, SF, RF, NF> Future for Retry<B, T, E, Fut, FutureFn, SF, RF, NF>
where
    B: Backoff,
    Fut: Future<Output = Result<T, E>>,
    FutureFn: FnMut() -> Fut,
    SF: Sleeper,
    RF: FnMut(&E) -> bool,
    NF: FnMut(&E, Duration),
{
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: This is safe because we don't move the `Retry` struct itself,
        // only its internal state.
        //
        // We do the exactly same thing like `pin_project` but without depending on it directly.
        let this = unsafe { self.get_unchecked_mut() };

        loop {
            match &mut this.state {
                State::Idle => {
                    let fut = (this.future_fn)();
                    this.state = State::Polling(fut);
                    continue;
                }
                State::Polling(fut) => {
                    // Safety: This is safe because we don't move the `Retry` struct and this fut,
                    // only its internal state.
                    //
                    // We do the exactly same thing like `pin_project` but without depending on it directly.
                    let mut fut = unsafe { Pin::new_unchecked(fut) };

                    match ready!(fut.as_mut().poll(cx)) {
                        Ok(v) => return Poll::Ready(Ok(v)),
                        Err(err) => {
                            // If input error is not retryable, return error directly.
                            if !(this.retryable_fn)(&err) {
                                return Poll::Ready(Err(err));
                            }
                            match this.backoff.next() {
                                None => return Poll::Ready(Err(err)),
                                Some(dur) => {
                                    (this.notify_fn)(&err, dur);
                                    this.state = State::Sleeping(this.sleep_fn.sleep(dur));
                                    continue;
                                }
                            }
                        }
                    }
                }
                State::Sleeping(sl) => {
                    // Safety: This is safe because we don't move the `Retry` struct and this fut,
                    // only its internal state.
                    //
                    // We do the exactly same thing like `pin_project` but without depending on it directly.
                    let mut sl = unsafe { Pin::new_unchecked(sl) };

                    ready!(sl.as_mut().poll(cx));
                    this.state = State::Idle;
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg(any(feature = "tokio-sleep", feature = "gloo-timers-sleep",))]
mod default_sleeper_tests {
    extern crate alloc;

    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::time::Duration;
    use tokio::sync::Mutex;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[cfg(not(target_arch = "wasm32"))]
    use tokio::test;

    use super::*;
    use crate::ExponentialBuilder;

    async fn always_error() -> anyhow::Result<()> {
        Err(anyhow::anyhow!("test_query meets error"))
    }

    #[test]
    async fn test_retry() {
        let result = always_error
            .retry(ExponentialBuilder::default().with_min_delay(Duration::from_millis(1)))
            .await;

        assert!(result.is_err());
        assert_eq!("test_query meets error", result.unwrap_err().to_string());
    }

    #[test]
    async fn test_retry_with_not_retryable_error() {
        let error_times = Mutex::new(0);

        let f = || async {
            let mut x = error_times.lock().await;
            *x += 1;
            Err::<(), anyhow::Error>(anyhow::anyhow!("not retryable"))
        };

        let backoff = ExponentialBuilder::default().with_min_delay(Duration::from_millis(1));
        let result = f
            .retry(backoff)
            // Only retry If error message is `retryable`
            .when(|e| e.to_string() == "retryable")
            .await;

        assert!(result.is_err());
        assert_eq!("not retryable", result.unwrap_err().to_string());
        // `f` always returns error "not retryable", so it should be executed
        // only once.
        assert_eq!(*error_times.lock().await, 1);
    }

    #[test]
    async fn test_retry_with_retryable_error() {
        let error_times = Mutex::new(0);

        let f = || async {
            let mut x = error_times.lock().await;
            *x += 1;
            Err::<(), anyhow::Error>(anyhow::anyhow!("retryable"))
        };

        let backoff = ExponentialBuilder::default().with_min_delay(Duration::from_millis(1));
        let result = f
            .retry(backoff)
            // Only retry If error message is `retryable`
            .when(|e| e.to_string() == "retryable")
            .await;

        assert!(result.is_err());
        assert_eq!("retryable", result.unwrap_err().to_string());
        // `f` always returns error "retryable", so it should be executed
        // 4 times (retry 3 times).
        assert_eq!(*error_times.lock().await, 4);
    }

    #[test]
    async fn test_fn_mut_when_and_notify() {
        let mut calls_retryable: Vec<()> = vec![];
        let mut calls_notify: Vec<()> = vec![];

        let f = || async { Err::<(), anyhow::Error>(anyhow::anyhow!("retryable")) };

        let backoff = ExponentialBuilder::default().with_min_delay(Duration::from_millis(1));
        let result = f
            .retry(backoff)
            .when(|_| {
                calls_retryable.push(());
                true
            })
            .notify(|_, _| {
                calls_notify.push(());
            })
            .await;

        assert!(result.is_err());
        assert_eq!("retryable", result.unwrap_err().to_string());
        // `f` always returns error "retryable", so it should be executed
        // 4 times (retry 3 times).
        assert_eq!(calls_retryable.len(), 4);
        assert_eq!(calls_notify.len(), 3);
    }
}

#[cfg(test)]
mod custom_sleeper_tests {
    extern crate alloc;

    use alloc::string::ToString;
    use core::{future::ready, time::Duration};

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[cfg(not(target_arch = "wasm32"))]
    use tokio::test;

    use super::*;
    use crate::ExponentialBuilder;

    async fn always_error() -> anyhow::Result<()> {
        Err(anyhow::anyhow!("test_query meets error"))
    }

    #[test]
    async fn test_retry_with_sleep() {
        let result = always_error
            .retry(ExponentialBuilder::default().with_min_delay(Duration::from_millis(1)))
            .sleep(|_| ready(()))
            .await;

        assert!(result.is_err());
        assert_eq!("test_query meets error", result.unwrap_err().to_string());
    }
}
