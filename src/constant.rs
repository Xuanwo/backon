use std::time::Duration;

/// ConstantBackoff provides backoff with constant delay and limited times.
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
/// use backon::ConstantBackoff;
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
///     let content = fetch.retry(ConstantBackoff::default()).await?;
///     println!("fetch succeeded: {}", content);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ConstantBackoff {
    delay: Duration,
    max_times: Option<usize>,

    attempts: usize,
}

impl Default for ConstantBackoff {
    fn default() -> Self {
        Self {
            delay: Duration::from_secs(1),
            max_times: Some(3),
            attempts: 0,
        }
    }
}

impl ConstantBackoff {
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
}

impl Iterator for ConstantBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        match self.max_times {
            None => Some(self.delay),
            Some(max_times) => {
                if self.attempts >= max_times {
                    None
                } else {
                    self.attempts += 1;
                    Some(self.delay)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ConstantBackoff;
    use std::time::Duration;

    #[test]
    fn test_constant_default() {
        let mut exp = ConstantBackoff::default();

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_constant_with_delay() {
        let mut exp = ConstantBackoff::default().with_delay(Duration::from_secs(2));

        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(Some(Duration::from_secs(2)), exp.next());
        assert_eq!(None, exp.next());
    }

    #[test]
    fn test_constant_with_times() {
        let mut exp = ConstantBackoff::default().with_max_times(1);

        assert_eq!(Some(Duration::from_secs(1)), exp.next());
        assert_eq!(None, exp.next());
    }
}
