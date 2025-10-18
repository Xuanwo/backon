use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use backon_macros::backon;

fn log_retry(err: &FetchError, dur: Duration) {
    println!("retrying after {dur:?}: {err:?}");
}

#[derive(Debug)]
struct FetchError {
    retryable: bool,
}

impl FetchError {
    fn retryable(err: &Self) -> bool {
        err.retryable
    }
}

static ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

#[backon(
    backoff = backon::ExponentialBuilder::default,
    sleep = tokio::time::sleep,
    when = FetchError::retryable,
    notify = log_retry
)]
async fn fetch_value() -> Result<&'static str, FetchError> {
    let attempt = ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    if attempt < 2 {
        Err(FetchError { retryable: true })
    } else {
        Ok("hello from async")
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    match fetch_value().await {
        Ok(value) => println!("{value}"),
        Err(err) => eprintln!("fetch failed: {err:?}"),
    }
}
