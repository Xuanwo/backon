use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use backon_macros::backon;

fn log_retry(err: &FetchError, dur: Duration) {
    println!("client retrying after {dur:?}: {err:?}");
}

#[derive(Debug)]
enum FetchError {
    Busy,
}

impl FetchError {
    fn retryable(err: &Self) -> bool {
        matches!(err, FetchError::Busy)
    }
}

struct Client {
    attempts: AtomicUsize,
}

impl Client {
    fn new() -> Self {
        Self {
            attempts: AtomicUsize::new(0),
        }
    }

    #[backon(
        backoff = backon::ExponentialBuilder::default,
        sleep = tokio::time::sleep,
        when = FetchError::retryable,
        notify = log_retry
    )]
    async fn fetch(&self) -> Result<&'static str, FetchError> {
        let attempt = self.attempts.fetch_add(1, Ordering::Relaxed);
        if attempt < 2 {
            Err(FetchError::Busy)
        } else {
            Ok("value from client")
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let client = Client::new();
    match client.fetch().await {
        Ok(value) => println!("{value}"),
        Err(err) => eprintln!("client failed: {err:?}"),
    }
}
