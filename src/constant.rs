use std::time::Duration;

use crate::backoff::BackoffBuilder;

/// ConstantBuilder is used to build [`ConstantBackoff`]
///
/// # Default
///
/// - delay: 1s
/// - max times: 3
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
///     let content = fetch.retry(&ConstantBuilder::default()).await?;
///     println!("fetch succeeded: {}", content);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ConstantBuilder {
    delay: Duration,
    max_times: Option<usize>,
    jitter: bool,
}

impl Default for ConstantBuilder {
    fn default() -> Self {
        Self {
            delay: Duration::from_secs(1),
            max_times: Some(3),
            jitter: false,
        }
    }
}

impl ConstantBuilder {
    /// Set delay of current backoff.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Set max times of current backoff.
    pub fn with_max_times(mut self, max_times: usize) -> Self {
        self.max_times = Some(max_times);
        self
    }

    /// Set jitter on
    pub fn with_jitter(mut self) -> Self {
        self.jitter = true;
        self
    }
}

impl BackoffBuilder for ConstantBuilder {
    type Backoff = ConstantBackoff;

    fn build(&self) -> Self::Backoff {
        ConstantBackoff {
            delay: self.delay,
            max_times: self.max_times,

            attempts: 0,
            jitter: self.jitter,
        }
    }
}

/// ConstantBackoff provides backoff with constant delay and limited times.
#[derive(Debug)]
pub struct ConstantBackoff {
    delay: Duration,
    max_times: Option<usize>,

    attempts: usize,
    jitter: bool,
}

impl Default for ConstantBackoff {
    fn default() -> Self {
        Self {
            delay: Duration::from_secs(1),
            max_times: Some(3),
            attempts: 0,
            jitter: false,
        }
    }
}

impl Iterator for ConstantBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        let delay = || match self.jitter {
            true => self.delay + self.delay.mul_f32(fastrand::f32()),
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
    use std::time::Duration;

    use crate::backoff::BackoffBuilder;
    use crate::constant::ConstantBuilder;

    #[test]
    fn test_constant_default() {
        let mut exp = ConstantBuilder::default().build();

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_constant_with_delay() {
        let mut exp = ConstantBuilder::default()
            .with_delay(Duration::from_secs(2))
            .build();

        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_constant_with_times() {
        let mut exp = ConstantBuilder::default().with_max_times(1).build();

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_constant_with_jitter() {
        let mut it = ConstantBuilder::default().with_jitter().build();

        let dur = it.next().unwrap();
        fastrand::seed(7);
        assert!(dur > Duration::from_secs(1));
    }
}
