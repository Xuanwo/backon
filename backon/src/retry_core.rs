use core::ops::ControlFlow;
use core::time::Duration;

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
        }
    }

    pub(crate) fn with_sleep<S>(self, sleep: S) -> RetryConfig<B, S, RetryFn, NotifyFn, AdjustFn> {
        RetryConfig {
            backoff: self.backoff,
            sleep,
            retryable: self.retryable,
            notify: self.notify,
            adjust: self.adjust,
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
        }
    }

    pub(crate) fn with_notify<N>(self, notify: N) -> RetryConfig<B, Sleep, RetryFn, N, AdjustFn> {
        RetryConfig {
            backoff: self.backoff,
            sleep: self.sleep,
            retryable: self.retryable,
            notify,
            adjust: self.adjust,
        }
    }

    pub(crate) fn with_adjust<A>(self, adjust: A) -> RetryConfig<B, Sleep, RetryFn, NotifyFn, A> {
        RetryConfig {
            backoff: self.backoff,
            sleep: self.sleep,
            retryable: self.retryable,
            notify: self.notify,
            adjust,
        }
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
