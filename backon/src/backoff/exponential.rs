use core::time::Duration;

use crate::backoff::BackoffBuilder;

/// ExponentialBuilder is used to construct an [`ExponentialBackoff`] that offers delays with exponential retries.
///
/// # Default
///
/// - jitter: false
/// - factor: 2
/// - min_delay: 1s
/// - max_delay: 60s
/// - max_times: 3
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
///     let content = fetch.retry(ExponentialBuilder::default()).await?;
///     println!("fetch succeeded: {}", content);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ExponentialBuilder {
    jitter: bool,
    factor: f32,
    min_delay: Duration,
    max_delay: Option<Duration>,
    max_times: Option<usize>,
}

impl Default for ExponentialBuilder {
    fn default() -> Self {
        Self {
            jitter: false,
            factor: 2.0,
            min_delay: Duration::from_secs(1),
            max_delay: Some(Duration::from_secs(60)),
            max_times: Some(3),
        }
    }
}

impl ExponentialBuilder {
    /// Set the jitter for the backoff.
    ///
    /// When jitter is enabled, [`ExponentialBackoff`] will add a random jitter within `(0, min_delay)`
    /// to the current delay.
    pub fn with_jitter(mut self) -> Self {
        self.jitter = true;
        self
    }

    /// Set the factor for the backoff.
    ///
    /// # Panics
    ///
    /// This function will panic if the input factor is less than `1.0`.
    pub fn with_factor(mut self, factor: f32) -> Self {
        debug_assert!(factor >= 1.0, "invalid factor that lower than 1");

        self.factor = factor;
        self
    }

    /// Set the minimum delay for the backoff.
    pub fn with_min_delay(mut self, min_delay: Duration) -> Self {
        self.min_delay = min_delay;
        self
    }

    /// Set the maximum delay for the backoff.
    ///
    /// The delay will not increase if the current delay exceeds the maximum delay.
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = Some(max_delay);
        self
    }

    /// Set no maximum delay for the backoff.
    ///
    /// The delay will keep increasing.
    ///
    /// _The delay will saturate at `Duration::MAX` which is an **unrealistic** delay._
    pub fn without_max_delay(mut self) -> Self {
        self.max_delay = None;
        self
    }

    /// Set the maximum number of attempts for the current backoff.
    ///
    /// The backoff will stop if the maximum number of attempts is reached.
    pub fn with_max_times(mut self, max_times: usize) -> Self {
        self.max_times = Some(max_times);
        self
    }

    /// Set no maximum number of attempts for the current backoff.
    ///
    /// The backoff will not stop by itself.
    ///
    /// _The backoff could stop reaching `usize::MAX` attempts but this is **unrealistic**._
    pub fn without_max_times(mut self) -> Self {
        self.max_times = None;
        self
    }
}

impl BackoffBuilder for ExponentialBuilder {
    type Backoff = ExponentialBackoff;

    fn build(self) -> Self::Backoff {
        ExponentialBackoff {
            jitter: self.jitter,
            factor: self.factor,
            min_delay: self.min_delay,
            max_delay: self.max_delay,
            max_times: self.max_times,

            current_delay: None,
            attempts: 0,
        }
    }
}

impl BackoffBuilder for &ExponentialBuilder {
    type Backoff = ExponentialBackoff;

    fn build(self) -> Self::Backoff {
        (*self).build()
    }
}

/// ExponentialBackoff provides a delay with exponential retries.
///
/// This backoff strategy is constructed by [`ExponentialBuilder`].
#[doc(hidden)]
#[derive(Debug)]
pub struct ExponentialBackoff {
    jitter: bool,
    factor: f32,
    min_delay: Duration,
    max_delay: Option<Duration>,
    max_times: Option<usize>,

    current_delay: Option<Duration>,
    attempts: usize,
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        if self.attempts >= self.max_times.unwrap_or(usize::MAX) {
            return None;
        }
        self.attempts += 1;

        let mut tmp_cur = match self.current_delay {
            None => {
                // If current_delay is None, it's must be the first time to retry.
                self.current_delay = Some(self.min_delay);
                self.min_delay
            }
            Some(mut cur) => {
                // If current delay larger than max delay, we should stop increment anymore.
                if let Some(max_delay) = self.max_delay {
                    if cur < max_delay {
                        cur = saturating_mul(cur, self.factor);
                    }
                    if cur > max_delay {
                        cur = max_delay;
                    }
                } else {
                    cur = saturating_mul(cur, self.factor);
                }
                self.current_delay = Some(cur);
                cur
            }
        };
        // If jitter is enabled, add random jitter based on min delay.
        if self.jitter {
            tmp_cur = tmp_cur.saturating_add(self.min_delay.mul_f32(fastrand::f32()));
        }
        Some(tmp_cur)
    }
}

#[inline]
pub(crate) fn saturating_mul(d: Duration, rhs: f32) -> Duration {
    Duration::try_from_secs_f32(rhs * d.as_secs_f32()).unwrap_or(Duration::MAX)
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use crate::BackoffBuilder;
    use crate::ExponentialBuilder;

    #[test]
    fn test_exponential_default() {
        let mut exp = ExponentialBuilder::default().build();

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(Some(Duration::from_secs(4)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_factor() {
        let mut exp = ExponentialBuilder::default().with_factor(1.5).build();

        assert_eq!(Some(Duration::from_secs_f32(1.0)), exp.next());
        assert_eq!(Some(Duration::from_secs_f32(1.5)), exp.next());
        assert_eq!(Some(Duration::from_secs_f32(2.25)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_jitter() {
        let mut exp = ExponentialBuilder::default().with_jitter().build();

        let v = exp.next().expect("value must valid");
        assert!(v >= Duration::from_secs(1), "current: {v:?}");
        assert!(v < Duration::from_secs(2), "current: {v:?}");

        let v = exp.next().expect("value must valid");
        assert!(v >= Duration::from_secs(2), "current: {v:?}");
        assert!(v < Duration::from_secs(4), "current: {v:?}");

        let v = exp.next().expect("value must valid");
        assert!(v >= Duration::from_secs(4), "current: {v:?}");
        assert!(v < Duration::from_secs(8), "current: {v:?}");

        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_min_delay() {
        let mut exp = ExponentialBuilder::default()
            .with_min_delay(Duration::from_millis(500))
            .build();

        assert_eq!(Some(Duration::from_millis(500)), exp.next());
        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_no_max_times_with_default() {
        let mut exp = ExponentialBuilder::default()
            .with_min_delay(Duration::from_secs(1))
            .with_factor(1_f32)
            .without_max_times()
            .build();

        // to fully test we would need to call this `usize::MAX`
        // which seems unreasonable for a test as it would take too long...
        for _ in 0..10_000 {
            assert_eq!(Some(Duration::from_secs(1)), exp.next());
        }
    }

    #[test]
    fn test_exponential_max_delay_with_default() {
        let mut exp = ExponentialBuilder::default()
            .with_max_delay(Duration::from_secs(2))
            .build();

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_no_max_delay_with_default() {
        let mut exp = ExponentialBuilder::default()
            .with_min_delay(Duration::from_secs(1))
            .with_factor(10_000_000_000_f32)
            .without_max_delay()
            .with_max_times(4)
            .build();

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(10_000_000_000)), exp.next());
        assert_eq!(Some(Duration::MAX), exp.next());
        assert_eq!(Some(Duration::MAX), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_max_delay_without_default_1() {
        let mut exp = ExponentialBuilder {
            jitter: false,
            factor: 10_000_000_000_f32,
            min_delay: Duration::from_secs(1),
            max_delay: None,
            max_times: None,
        }
        .build();

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(10_000_000_000)), exp.next());
        assert_eq!(Some(Duration::MAX), exp.next());
        assert_eq!(Some(Duration::MAX), exp.next());
    }

    #[test]
    fn test_exponential_max_delay_without_default_2() {
        let mut exp = ExponentialBuilder {
            jitter: true,
            factor: 10_000_000_000_f32,
            min_delay: Duration::from_secs(10_000_000_000),
            max_delay: None,
            max_times: Some(2),
        }
        .build();
        let v = exp.next().expect("value must valid");
        assert!(v >= Duration::from_secs(10_000_000_000), "current: {v:?}");
        assert!(v < Duration::from_secs(20_000_000_000), "current: {v:?}");
        assert_eq!(Some(Duration::MAX), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_max_delay_without_default_3() {
        let mut exp = ExponentialBuilder {
            jitter: false,
            factor: 10_000_000_000_f32,
            min_delay: Duration::from_secs(10_000_000_000),
            max_delay: Some(Duration::from_secs(60_000_000_000)),
            max_times: Some(3),
        }
        .build();
        assert_eq!(Some(Duration::from_secs(10_000_000_000)), exp.next());
        assert_eq!(Some(Duration::from_secs(60_000_000_000)), exp.next());
        assert_eq!(Some(Duration::from_secs(60_000_000_000)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_max_times() {
        let mut exp = ExponentialBuilder::default().with_max_times(1).build();

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(None, exp.next());
    }
}
