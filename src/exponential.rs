use rand::Rng;
use std::time::Duration;

/// Exponential backoff implementation.
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
/// use backon::ExponentialBackoff;
/// use backon::Retryable;
///
/// async fn fetch() -> Result<String> {
///     Ok(reqwest::get("https://www.rust-lang.org")
///         .await?
///         .text()
///         .await?)
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let content = fetch.retry(ExponentialBackoff::default()).await?;
///     println!("fetch succeeded: {}", content);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    jitter: bool,
    factor: f32,
    min_delay: Duration,
    max_delay: Option<Duration>,
    max_times: Option<usize>,

    current_delay: Option<Duration>,
    attempts: usize,
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            jitter: false,
            factor: 2.0,
            min_delay: Duration::from_secs(1),
            max_delay: Some(Duration::from_secs(60)),
            max_times: Some(3),

            current_delay: None,
            attempts: 0,
        }
    }
}

impl ExponentialBackoff {
    /// Set jitter of current backoff.
    ///
    /// If jitter is enabled, ExponentialBackoff will add a random jitter in `[0, min_delay)
    /// to current delay.
    pub fn with_jitter(mut self) -> Self {
        self.jitter = true;
        self
    }

    /// Set factor of current backoff.
    ///
    /// # Panics
    ///
    /// This function will panic if input factor smaller than `1.0`.
    pub fn with_factor(mut self, factor: f32) -> Self {
        debug_assert!(factor > 1.0, "invalid factor that lower than 1");

        self.factor = factor;
        self
    }

    /// Set min_delay of current backoff.
    pub fn with_min_delay(mut self, min_delay: Duration) -> Self {
        self.min_delay = min_delay;
        self
    }

    /// Set max_delay of current backoff.
    ///
    /// Delay will not increasing if current delay is larger than max_delay.
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = Some(max_delay);
        self
    }

    /// Set max_times of current backoff.
    ///
    /// Backoff will return `None` if max times is reaching.
    pub fn with_max_times(mut self, max_times: usize) -> Self {
        self.max_times = Some(max_times);
        self
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        if self.attempts >= self.max_times.unwrap_or(usize::MAX) {
            return None;
        }
        self.attempts += 1;

        match self.current_delay {
            None => {
                // If current_delay is None, it's must be the first time to retry.
                let mut cur = self.min_delay;
                self.current_delay = Some(cur);

                // If jitter is enabled, add random jitter based on min delay.
                if self.jitter {
                    cur += self
                        .min_delay
                        .mul_f32(rand::thread_rng().gen_range(0.0..1.0));
                }

                Some(cur)
            }
            Some(mut cur) => {
                // If current delay larger than max delay, we should stop increment anymore.
                if let Some(max_delay) = self.max_delay {
                    if cur < max_delay {
                        cur = cur.mul_f32(self.factor);
                    }
                }
                self.current_delay = Some(cur);

                // If jitter is enabled, add random jitter based on min delay.
                if self.jitter {
                    cur += self
                        .min_delay
                        .mul_f32(rand::thread_rng().gen_range(0.0..1.0));
                }

                Some(cur)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::exponential::ExponentialBackoff;
    use std::time::Duration;

    #[test]
    fn test_exponential_default() {
        let mut exp = ExponentialBackoff::default();

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(Some(Duration::from_secs(4)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_factor() {
        let mut exp = ExponentialBackoff::default().with_factor(1.5);

        assert_eq!(Some(Duration::from_secs_f32(1.0)), exp.next());
        assert_eq!(Some(Duration::from_secs_f32(1.5)), exp.next());
        assert_eq!(Some(Duration::from_secs_f32(2.25)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_jitter() {
        let mut exp = ExponentialBackoff::default().with_jitter();

        let v = exp.next().expect("value must valid");
        assert!(v >= Duration::from_secs(1), "current: {:?}", v);
        assert!(v < Duration::from_secs(2), "current: {:?}", v);

        let v = exp.next().expect("value must valid");
        assert!(v >= Duration::from_secs(2), "current: {:?}", v);
        assert!(v < Duration::from_secs(4), "current: {:?}", v);

        let v = exp.next().expect("value must valid");
        assert!(v >= Duration::from_secs(4), "current: {:?}", v);
        assert!(v < Duration::from_secs(8), "current: {:?}", v);

        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_min_delay() {
        let mut exp = ExponentialBackoff::default().with_min_delay(Duration::from_millis(500));

        assert_eq!(Some(Duration::from_millis(500)), exp.next());
        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_max_delay() {
        let mut exp = ExponentialBackoff::default().with_max_delay(Duration::from_secs(2));

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_exponential_max_times() {
        let mut exp = ExponentialBackoff::default().with_max_times(1);

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(None, exp.next());
    }
}
