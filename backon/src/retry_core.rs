use core::ops::ControlFlow;
use core::time::Duration;

#[cfg(all(
    feature = "std",
    not(all(target_arch = "wasm32", target_os = "unknown"))
))]
use std::time::Instant;
#[cfg(all(feature = "std", target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

use crate::Backoff;

pub(crate) fn always_retry<E>(_: &E) -> bool {
    true
}

pub(crate) fn noop_notify<E>(_: &E, _: Duration) {}

pub(crate) fn identity_adjust<E>(_: &E, dur: Option<Duration>) -> Option<Duration> {
    dur
}

/// Shared configuration for retry executors.
pub(crate) struct RetryConfig<B, Sleep, RetryFn, NotifyFn, AdjustFn> {
    pub(crate) backoff: B,
    pub(crate) sleep: Sleep,
    pub(crate) retryable: RetryFn,
    pub(crate) notify: NotifyFn,
    pub(crate) adjust: AdjustFn,
    timer: RetryTimer,
}

#[derive(Default)]
struct RetryTimer {
    #[cfg(feature = "std")]
    max_elapsed_time: Option<Duration>,
    #[cfg(feature = "std")]
    started_at: Option<Instant>,
}

impl<B, Sleep, RetryFn, NotifyFn, AdjustFn> RetryConfig<B, Sleep, RetryFn, NotifyFn, AdjustFn> {
    pub(crate) fn new(
        backoff: B,
        sleep: Sleep,
        retryable: RetryFn,
        notify: NotifyFn,
        adjust: AdjustFn,
    ) -> Self {
        RetryConfig {
            backoff,
            sleep,
            retryable,
            notify,
            adjust,
            timer: RetryTimer::default(),
        }
    }

    pub(crate) fn with_sleep<S>(self, sleep: S) -> RetryConfig<B, S, RetryFn, NotifyFn, AdjustFn> {
        RetryConfig {
            backoff: self.backoff,
            sleep,
            retryable: self.retryable,
            notify: self.notify,
            adjust: self.adjust,
            timer: self.timer,
        }
    }

    pub(crate) fn with_retryable<R>(
        self,
        retryable: R,
    ) -> RetryConfig<B, Sleep, R, NotifyFn, AdjustFn> {
        RetryConfig {
            backoff: self.backoff,
            sleep: self.sleep,
            retryable,
            notify: self.notify,
            adjust: self.adjust,
            timer: self.timer,
        }
    }

    pub(crate) fn with_notify<N>(self, notify: N) -> RetryConfig<B, Sleep, RetryFn, N, AdjustFn> {
        RetryConfig {
            backoff: self.backoff,
            sleep: self.sleep,
            retryable: self.retryable,
            notify,
            adjust: self.adjust,
            timer: self.timer,
        }
    }

    pub(crate) fn with_adjust<A>(self, adjust: A) -> RetryConfig<B, Sleep, RetryFn, NotifyFn, A> {
        RetryConfig {
            backoff: self.backoff,
            sleep: self.sleep,
            retryable: self.retryable,
            notify: self.notify,
            adjust,
            timer: self.timer,
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn with_max_elapsed_time(mut self, max_elapsed_time: Option<Duration>) -> Self {
        self.timer.max_elapsed_time = max_elapsed_time;
        self
    }

    pub(crate) fn start(&mut self) {
        #[cfg(feature = "std")]
        if self.timer.max_elapsed_time.is_some() && self.timer.started_at.is_none() {
            self.timer.started_at = Some(Instant::now());
        }
    }

    #[cfg(all(test, feature = "std"))]
    pub(crate) fn timer_started(&self) -> bool {
        self.timer.started_at.is_some()
    }

    fn has_elapsed(&self) -> bool {
        #[cfg(feature = "std")]
        {
            self.timer
                .max_elapsed_time
                .zip(self.timer.started_at)
                .is_some_and(|(max_elapsed_time, started_at)| {
                    started_at.elapsed() >= max_elapsed_time
                })
        }

        #[cfg(not(feature = "std"))]
        false
    }
}

impl<B, Sleep, RetryFn, NotifyFn, AdjustFn> RetryConfig<B, Sleep, RetryFn, NotifyFn, AdjustFn>
where
    B: Backoff,
{
    pub(crate) fn decide<E>(&mut self, err: &E) -> ControlFlow<(), Duration>
    where
        RetryFn: FnMut(&E) -> bool,
        NotifyFn: FnMut(&E, Duration),
        AdjustFn: FnMut(&E, Option<Duration>) -> Option<Duration>,
    {
        if !(self.retryable)(err) {
            return ControlFlow::Break(());
        }

        if self.has_elapsed() {
            return ControlFlow::Break(());
        }

        let candidate = self.backoff.next();
        match (self.adjust)(err, candidate) {
            Some(dur) => {
                (self.notify)(err, dur);
                ControlFlow::Continue(dur)
            }
            None => ControlFlow::Break(()),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::BackoffBuilder;
    use crate::ConstantBuilder;

    #[test]
    fn test_elapsed_limit_before_backoff() {
        let backoff = ConstantBuilder::default()
            .with_delay(Duration::from_secs(1))
            .with_max_times(1)
            .build();
        let mut config = RetryConfig::new(
            backoff,
            (),
            always_retry::<()>,
            noop_notify::<()>,
            identity_adjust::<()>,
        )
        .with_max_elapsed_time(Some(Duration::ZERO));
        config.start();

        assert_eq!(config.decide(&()), ControlFlow::Break(()));

        config.timer.max_elapsed_time = Some(Duration::MAX);
        assert_eq!(
            config.decide(&()),
            ControlFlow::Continue(Duration::from_secs(1))
        );
    }

    #[test]
    fn test_elapsed_limit_with_longer_delay() {
        let backoff = ConstantBuilder::default()
            .with_delay(Duration::from_secs(2 * 60 * 60))
            .with_max_times(1)
            .build();
        let mut config = RetryConfig::new(
            backoff,
            (),
            always_retry::<()>,
            noop_notify::<()>,
            identity_adjust::<()>,
        )
        .with_max_elapsed_time(Some(Duration::from_secs(60 * 60)));
        config.start();

        assert_eq!(
            config.decide(&()),
            ControlFlow::Continue(Duration::from_secs(2 * 60 * 60))
        );
    }
}
