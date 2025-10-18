use std::sync::atomic::{AtomicUsize, Ordering};

use backon_macros::backon;

#[derive(Debug)]
struct StoreError;

static ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

fn should_retry(_: &StoreError) -> bool {
    true
}

#[backon(when = should_retry)]
fn store_value() -> Result<&'static str, StoreError> {
    let attempt = ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    if attempt < 3 {
        Err(StoreError)
    } else {
        Ok("stored")
    }
}

fn main() {
    match store_value() {
        Ok(value) => println!("{value}"),
        Err(err) => eprintln!("store failed: {err:?}"),
    }
}
