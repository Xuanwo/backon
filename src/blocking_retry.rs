use std::thread;
use std::time::Duration;

use crate::backoff::BackoffBuilder;
use crate::Backoff;

/// BlockingRetryable will add retry support for functions.
///
/// For example:
///
/// - Functions without extra args:
///
/// ```ignore
/// fn fetch() -> Result<String> {
///     Ok("hello, world!".to_string())
/// }
/// ```
///
/// - Closures
///
/// ```ignore
/// || {
///     Ok("hello, world!".to_string())
/// }
/// ```
///
/// # Example
///
/// ```no_run
/// use anyhow::Result;
/// use backon::BlockingRetryable;
/// use backon::ExponentialBuilder;
///
/// fn fetch() -> Result<String> {
///     Ok("hello, world!".to_string())
/// }
///
/// fn main() -> Result<()> {
///     let content = fetch.retry(&ExponentialBuilder::default()).call()?;
///     println!("fetch succeeded: {}", content);
///
///     Ok(())
/// }
/// ```
pub trait BlockingRetryable<B: BackoffBuilder, T, E, F: FnMut() -> Result<T, E>> {
    /// Generate a new retry
    fn retry(self, builder: &B) -> BlockingRetry<B::Backoff, T, E, F>;
}

impl<B, T, E, F> BlockingRetryable<B, T, E, F> for F
where
    B: BackoffBuilder,
    F: FnMut() -> Result<T, E>,
{
    fn retry(self, builder: &B) -> BlockingRetry<B::Backoff, T, E, F> {
        BlockingRetry::new(self, builder.build())
    }
}

/// Retry struct generated by [`Retryable`].
pub struct BlockingRetry<B: Backoff, T, E, F: FnMut() -> Result<T, E>, RF = (), NF = ()> {
    backoff: B,
    retryable: RF,
    notify: NF,
    f: F,
}

impl<B, T, E, F> BlockingRetry<B, T, E, F>
where
    B: Backoff,
    F: FnMut() -> Result<T, E>,
{
    /// Create a new retry.
    fn new(f: F, backoff: B) -> Self {
        BlockingRetry {
            backoff,
            retryable: (),
            notify: (),
            f,
        }
    }

    /// Call the retried function.
    ///
    /// TODO: implment [`std::ops::FnOnce`] after it stable.
    pub fn call(mut self) -> Result<T, E> {
        loop {
            let result = (self.f)();

            match result {
                Ok(v) => return Ok(v),
                Err(err) => match self.backoff.next() {
                    None => return Err(err),
                    Some(dur) => {
                        thread::sleep(dur);
                    }
                },
            }
        }
    }
}

impl<B, T, E, F, NF> BlockingRetry<B, T, E, F, (), NF>
where
    B: Backoff,
    F: FnMut() -> Result<T, E>,
    NF: FnMut(&E, Duration),
{
    /// Call the retried function.
    ///
    /// TODO: implment [`std::ops::FnOnce`] after it stable.
    pub fn call(mut self) -> Result<T, E> {
        loop {
            let result = (self.f)();

            match result {
                Ok(v) => return Ok(v),
                Err(err) => match self.backoff.next() {
                    None => return Err(err),
                    Some(dur) => {
                        (self.notify)(&err, dur);
                        thread::sleep(dur);
                    }
                },
            }
        }
    }
}

impl<B, T, E, F, RF> BlockingRetry<B, T, E, F, RF>
where
    B: Backoff,
    F: FnMut() -> Result<T, E>,
    RF: FnMut(&E) -> bool,
{
    /// Call the retried function.
    ///
    /// TODO: implment [`std::ops::FnOnce`] after it stable.
    pub fn call(mut self) -> Result<T, E> {
        loop {
            let result = (self.f)();

            match result {
                Ok(v) => return Ok(v),
                Err(err) => {
                    if !(self.retryable)(&err) {
                        return Err(err);
                    }

                    match self.backoff.next() {
                        None => return Err(err),
                        Some(dur) => {
                            thread::sleep(dur);
                        }
                    }
                }
            }
        }
    }
}

impl<B, T, E, F, RF, NF> BlockingRetry<B, T, E, F, RF, NF>
where
    B: Backoff,
    F: FnMut() -> Result<T, E>,
    RF: FnMut(&E) -> bool,
    NF: FnMut(&E, Duration),
{
    /// Call the retried function.
    ///
    /// TODO: implment [`std::ops::FnOnce`] after it stable.
    pub fn call(mut self) -> Result<T, E> {
        loop {
            let result = (self.f)();

            match result {
                Ok(v) => return Ok(v),
                Err(err) => {
                    if !(self.retryable)(&err) {
                        return Err(err);
                    }

                    match self.backoff.next() {
                        None => return Err(err),
                        Some(dur) => {
                            (self.notify)(&err, dur);
                            thread::sleep(dur);
                        }
                    }
                }
            }
        }
    }
}

impl<B, T, E, F, NF> BlockingRetry<B, T, E, F, (), NF>
where
    B: Backoff,
    F: FnMut() -> Result<T, E>,
{
    /// Set the conditions for retrying.
    ///
    /// If not specified, we treat all errors as retryable.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use anyhow::Result;
    /// use backon::BlockingRetryable;
    /// use backon::ExponentialBuilder;
    ///
    /// fn fetch() -> Result<String> {
    ///     Ok("hello, world!".to_string())
    /// }
    ///
    /// fn main() -> Result<()> {
    ///     let retry = fetch
    ///         .retry(&ExponentialBuilder::default())
    ///         .when(|e| e.to_string() == "EOF");
    ///     let content = retry.call()?;
    ///     println!("fetch succeeded: {}", content);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn when<RF: FnMut(&E) -> bool>(self, retryable: RF) -> BlockingRetry<B, T, E, F, RF, NF> {
        BlockingRetry {
            backoff: self.backoff,
            retryable,
            notify: self.notify,
            f: self.f,
        }
    }
}

impl<B, T, E, F, RF> BlockingRetry<B, T, E, F, RF, ()>
where
    B: Backoff,
    F: FnMut() -> Result<T, E>,
{
    /// Set to notify for everything retrying.
    ///
    /// If not specified, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use anyhow::Result;
    /// use backon::BlockingRetryable;
    /// use backon::ExponentialBuilder;
    ///
    /// fn fetch() -> Result<String> {
    ///     Ok("hello, world!".to_string())
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let retry = fetch.retry(&ExponentialBuilder::default()).notify(
    ///         |err: &anyhow::Error, dur: Duration| {
    ///             println!("retrying error {:?} with sleeping {:?}", err, dur);
    ///         },
    ///     );
    ///     let content = retry.call()?;
    ///     println!("fetch succeeded: {}", content);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn notify<NF: FnMut(&E, Duration)>(self, notify: NF) -> BlockingRetry<B, T, E, F, RF, NF> {
        BlockingRetry {
            backoff: self.backoff,
            retryable: self.retryable,
            notify,
            f: self.f,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use super::*;
    use crate::ExponentialBuilder;

    fn always_error() -> anyhow::Result<()> {
        Err(anyhow::anyhow!("test_query meets error"))
    }

    #[test]
    fn test_retry() -> anyhow::Result<()> {
        let result = always_error
            .retry(&ExponentialBuilder::default().with_min_delay(Duration::from_millis(1)))
            .call();

        assert!(result.is_err());
        assert_eq!("test_query meets error", result.unwrap_err().to_string());
        Ok(())
    }

    #[test]
    fn test_retry_with_not_retryable_error() -> anyhow::Result<()> {
        let error_times = Mutex::new(0);

        let f = || {
            let mut x = error_times.lock().unwrap();
            *x += 1;
            Err::<(), anyhow::Error>(anyhow::anyhow!("not retryable"))
        };

        let backoff = ExponentialBuilder::default().with_min_delay(Duration::from_millis(1));
        let result = f
            .retry(&backoff)
            // Only retry If error message is `retryable`
            .when(|e| e.to_string() == "retryable")
            .call();

        assert!(result.is_err());
        assert_eq!("not retryable", result.unwrap_err().to_string());
        // `f` always returns error "not retryable", so it should be executed
        // only once.
        assert_eq!(*error_times.lock().unwrap(), 1);
        Ok(())
    }

    #[test]
    fn test_retry_with_retryable_error() -> anyhow::Result<()> {
        let error_times = Mutex::new(0);

        let f = || {
            println!("I have been called!");
            let mut x = error_times.lock().unwrap();
            *x += 1;
            Err::<(), anyhow::Error>(anyhow::anyhow!("retryable"))
        };

        let backoff = ExponentialBuilder::default().with_min_delay(Duration::from_millis(1));
        let result = f
            .retry(&backoff)
            // Only retry If error message is `retryable`
            .when(|e| e.to_string() == "retryable")
            .call();

        assert!(result.is_err());
        assert_eq!("retryable", result.unwrap_err().to_string());
        // `f` always returns error "retryable", so it should be executed
        // 4 times (retry 3 times).
        assert_eq!(*error_times.lock().unwrap(), 4);
        Ok(())
    }
}
