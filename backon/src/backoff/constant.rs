use core::time::Duration;

use crate::backoff::BackoffBuilder;

use super::Random;

/// ConstantBuilder is used to create a [`ConstantBackoff`], providing a steady delay with a fixed number of retries.
///
/// # Default
///
/// - delay: 1s
/// - max_times: 3
///
/// # Examples
///
/// ```no_run
/// use anyhow::Result;
/// use backon::ConstantBuilder;
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
///     let content = fetch.retry(ConstantBuilder::default()).await?;
///     println!("fetch succeeded: {}", content);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ConstantBuilder {
    delay: Duration,
    max_times: Option<usize>,
    jitter: bool,
    seed: u64,
}

impl Default for ConstantBuilder {
    fn default() -> Self {
        Self {
            delay: Duration::from_secs(1),
            max_times: Some(3),
            jitter: false,
            seed: 0x2fdb0020ffc7722b,
        }
    }
}

impl ConstantBuilder {
    /// Set the delay for the backoff.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Set the maximum number of attempts to be made.
    pub fn with_max_times(mut self, max_times: usize) -> Self {
        self.max_times = Some(max_times);
        self
    }

    /// Enable jitter for the backoff.
    ///
    /// Jitter is a random value added to the delay to prevent a thundering herd problem.
    pub fn with_jitter(mut self) -> Self {
        self.jitter = true;
        self
    }

    /// Set the seed value for the jitter random number generator.
    pub fn with_jitter_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set no max times for the backoff.
    ///
    /// The backoff will not stop by itself.
    ///
    /// _The backoff could stop reaching `usize::MAX` attempts but this is **unrealistic**._
    pub fn without_max_times(mut self) -> Self {
        self.max_times = None;
        self
    }
}

impl BackoffBuilder for ConstantBuilder {
    type Backoff = ConstantBackoff;

    fn build(self) -> Self::Backoff {
        ConstantBackoff {
            delay: self.delay,
            max_times: self.max_times,

            attempts: 0,
            jitter: self.jitter,
            #[cfg(not(feature = "std"))]
            seed: self.seed,
        }
    }
}

impl BackoffBuilder for &ConstantBuilder {
    type Backoff = ConstantBackoff;

    fn build(self) -> Self::Backoff {
        (*self).build()
    }
}

/// ConstantBackoff offers a consistent delay with a limited number of retries.
///
/// This backoff strategy is constructed by [`ConstantBuilder`].
#[doc(hidden)]
#[derive(Debug)]
pub struct ConstantBackoff {
    delay: Duration,
    max_times: Option<usize>,

    attempts: usize,
    jitter: bool,
    #[cfg(not(feature = "std"))]
    seed: u64,
}

impl Random for ConstantBackoff {
    #[cfg(not(feature = "std"))]
    fn seed(&self) -> u64 {
        self.seed
    }

    #[cfg(not(feature = "std"))]
    fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }
}

impl Iterator for ConstantBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        let jitter = self.jitter();
        let delay = || match self.jitter {
            true => self.delay + self.delay.mul_f32(jitter),
            false => self.delay,
        };
        match self.max_times {
            None => Some(delay()),
            Some(max_times) => {
                if self.attempts >= max_times {
                    None
                } else {
                    self.attempts += 1;
                    Some(delay())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use crate::BackoffBuilder;
    use crate::ConstantBuilder;

    #[test]
    fn test_constant_default() {
        let mut it = ConstantBuilder::default().build();

        assert_eq!(Some(Duration::from_secs(1)), it.next());
        assert_eq!(Some(Duration::from_secs(1)), it.next());
        assert_eq!(Some(Duration::from_secs(1)), it.next());
        assert_eq!(None, it.next());
    }

    #[test]
    fn test_constant_with_delay() {
        let mut it = ConstantBuilder::default()
            .with_delay(Duration::from_secs(2))
            .build();

        assert_eq!(Some(Duration::from_secs(2)), it.next());
        assert_eq!(Some(Duration::from_secs(2)), it.next());
        assert_eq!(Some(Duration::from_secs(2)), it.next());
        assert_eq!(None, it.next());
    }

    #[test]
    fn test_constant_with_times() {
        let mut it = ConstantBuilder::default().with_max_times(1).build();

        assert_eq!(Some(Duration::from_secs(1)), it.next());
        assert_eq!(None, it.next());
    }

    #[test]
    fn test_constant_with_jitter() {
        let mut it = ConstantBuilder::default().with_jitter().build();

        let dur = it.next().unwrap();
        fastrand::seed(7);
        assert!(dur > Duration::from_secs(1));
    }

    #[test]
    fn test_constant_without_max_times() {
        let mut it = ConstantBuilder::default().without_max_times().build();

        for _ in 0..10_000 {
            assert_eq!(Some(Duration::from_secs(1)), it.next());
        }
    }
}
