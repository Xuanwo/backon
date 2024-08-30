use core::{
    future::{Future, Ready},
    time::Duration,
};

/// A sleeper is used to generate a future that completes after a specified duration.
pub trait Sleeper: 'static {
    /// The future returned by the `sleep` method.
    type Sleep: Future<Output = ()>;

    /// Create a future that completes after a set period.
    fn sleep(&self, dur: Duration) -> Self::Sleep;
}

/// A stub trait allowing non-[`Sleeper`] types to be used as a generic parameter in [`Retry`][crate::Retry].
/// It does not provide actual functionality.
#[doc(hidden)]
pub trait MaybeSleeper: 'static {
    type Sleep: Future<Output = ()>;
}

/// All `Sleeper` will implement  `MaybeSleeper`, but not vice versa.
impl<T: Sleeper + ?Sized> MaybeSleeper for T {
    type Sleep = <T as Sleeper>::Sleep;
}

/// All `Fn(Duration) -> impl Future<Output = ()>` implements `Sleeper`.
impl<F: Fn(Duration) -> Fut + 'static, Fut: Future<Output = ()>> Sleeper for F {
    type Sleep = Fut;

    fn sleep(&self, dur: Duration) -> Self::Sleep {
        self(dur)
    }
}

/// The default implementation of `Sleeper` when no features are enabled.
///
/// It will fail to compile if a containing [`Retry`][crate::Retry] is `.await`ed without calling [`Retry::sleep`][crate::Retry::sleep] to provide a valid sleeper.
#[cfg(all(not(feature = "tokio-sleep"), not(feature = "gloo-timers-sleep")))]
pub type DefaultSleeper = PleaseEnableAFeatureOrProvideACustomSleeper;
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

/// A placeholder type that does not implement [`Sleeper`] and will therefore fail to compile if used as one.
///
/// Users should enable a feature of this crate that provides a valid [`Sleeper`] implementation when this type appears in compilation errors. Alternatively, a custom [`Sleeper`] implementation should be provided where necessary, such as in [`crate::Retry::sleeper`].
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PleaseEnableAFeatureOrProvideACustomSleeper;

/// Implement `MaybeSleeper` but not `Sleeper`.
impl MaybeSleeper for PleaseEnableAFeatureOrProvideACustomSleeper {
    type Sleep = Ready<()>;
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
