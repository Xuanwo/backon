use std::{
    future::{Future, Ready},
    time::Duration,
};

#[cfg(doc)]
use crate::Retry;

/// A sleeper is used to generate a future that completes after a specified duration.
pub trait Sleeper: 'static {
    /// The future returned by the `sleep` method.
    type Sleep: Future<Output = ()>;

    /// Create a future that completes after a set period.
    fn sleep(&self, dur: Duration) -> Self::Sleep;
}

/// A stub trait allowing non-[`Sleeper`] types to be used as a generic parameter in [`Retry`].
/// It does not provide actual functionality.
#[doc(hidden)]
pub trait MayBeDefaultSleeper: 'static {
    type Sleep: Future<Output = ()>;
}

/// The default implementation of `Sleeper` when no features are enabled.
/// It will fail to compile if a containing [`Retry`] is `.await`ed without calling [`Retry::sleep`]
/// to provide a valid sleeper.
#[cfg(all(not(feature = "tokio-sleep"), not(feature = "gloo-timers-sleep")))]
pub type DefaultSleeper = PleaseEnableAFeatureForSleeper;
/// The default implementation of `Sleeper` while feature `tokio-sleep` enabled.
///
/// it uses `tokio::time::sleep`.
#[cfg(all(not(target_arch = "wasm32"), feature = "tokio-sleep"))]
pub type DefaultSleeper = TokioSleeper;
/// The default implementation of `Sleeper` while feature `gloo-timers-sleep` enabled.
///
/// It uses `gloo_timers::sleep::sleep`.
#[cfg(all(target_arch = "wasm32", feature = "gloo-timers-sleep"))]
pub type DefaultSleeper = GlooTimersSleep;

/// A stub type that does not implement [`Sleeper`] and hence will fail to compile if used as a
/// sleeper.
///
/// Users are expected to enable a feature of this crate that provides a valid implementation of
/// [`Sleeper`] when they see this type appearing in compilation errors. Otherwise, a custom [`Sleeper`]
/// implementation should be provided where needed, such as [`Retry::sleeper`].
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PleaseEnableAFeatureForSleeper;

impl MayBeDefaultSleeper for PleaseEnableAFeatureForSleeper {
    type Sleep = Ready<()>;
}

impl<T: Sleeper + ?Sized> MayBeDefaultSleeper for T {
    type Sleep = <T as Sleeper>::Sleep;
}

impl<F: Fn(Duration) -> Fut + 'static, Fut: Future<Output = ()>> Sleeper for F {
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
