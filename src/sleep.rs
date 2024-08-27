use std::{
    future::{Future, Ready},
    time::Duration,
};

/// A sleeper is used to generate a future that completes after a specified duration.
pub trait Sleeper {
    /// The future returned by the `sleep` method.
    type Sleep: Future<Output = ()>;

    /// Create a future that completes after a set period.
    fn sleep(&self, dur: Duration) -> Self::Sleep;
}

/// The default implementation of `Sleeper`.
///
/// - Under `tokio-sleep` feature, it uses `tokio::time::sleep`.
/// - Under `gloo-timers-sleep` feature, it uses `gloo_timers::sleep::sleep`.
#[cfg(all(not(feature = "tokio-sleep"), not(feature = "gloo-timers-sleep")))]
pub type DefaultSleeper = ();
/// The default implementation of `Sleeper` based on enabled feature flag.
///
/// Under `tokio-sleep` feature, it uses `tokio::time::sleep`.
#[cfg(all(not(target_arch = "wasm32"), feature = "tokio-sleep"))]
pub type DefaultSleeper = TokioSleeper;
/// The default implementation of `Sleeper` based on enabled feature flag.
///
/// Under `gloo-timers-sleep` feature, it uses `gloo_timers::sleep::sleep`.
#[cfg(all(target_arch = "wasm32", feature = "gloo-timers-sleep"))]
pub type DefaultSleeper = GlooTimersSleep;

impl Sleeper for () {
    type Sleep = Ready<()>;

    fn sleep(&self, _: Duration) -> Self::Sleep {
        panic!("no sleeper has been configured, consider enabling features or provide a custom implementation")
    }
}

impl<F: Fn(Duration) -> Fut, Fut: Future<Output = ()>> Sleeper for F {
    type Sleep = Fut;

    fn sleep(&self, dur: Duration) -> Self::Sleep {
        self(dur)
    }
}

/// The default implementation of `Sleeper` uses `tokio::time::sleep`.
///
/// It will adhere to [pausing/auto-advancing](https://docs.rs/tokio/latest/tokio/time/fn.pause.html)
/// in Tokio's Runtime semantics, if enabled.
#[cfg(all(not(target_arch = "wasm32"), feature = "tokio-sleep"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioSleeper;

#[cfg(all(not(target_arch = "wasm32"), feature = "tokio-sleep"))]
impl Sleeper for TokioSleeper {
    type Sleep = tokio::time::Sleep;

    fn sleep(&self, dur: Duration) -> Self::Sleep {
        tokio::time::sleep(dur)
    }
}

/// The default implementation of `Sleeper` utilizes `gloo_timers::future::sleep`.
#[cfg(all(target_arch = "wasm32", feature = "gloo-timers-sleep"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct GlooTimersSleep;

#[cfg(all(target_arch = "wasm32", feature = "gloo-timers-sleep"))]
impl Sleeper for GlooTimersSleep {
    type Sleep = gloo_timers::future::TimeoutFuture;

    fn sleep(&self, dur: Duration) -> Self::Sleep {
        gloo_timers::future::sleep(dur)
    }
}
