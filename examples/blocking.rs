use anyhow::Result;

// For more examples, please see: https://docs.rs/backon/#examples

fn fetch() -> Result<String> {
    Ok("hello, world!".to_string())
}

// this example does not run on wasm32-unknown-unknown
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    use backon::BlockingRetryable;

    let content = fetch.retry(&backon::ExponentialBuilder::default()).call()?;
    println!("fetch succeeded: {}", content);

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
