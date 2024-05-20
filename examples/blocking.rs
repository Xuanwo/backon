use anyhow::Result;
use backon::BlockingRetryable;
use backon::ExponentialBuilder;

// For more examples, please see: https://docs.rs/backon/#examples

fn fetch() -> Result<String> {
    Ok("hello, world!".to_string())
}

fn main() -> Result<()> {
    let content = fetch.retry(&ExponentialBuilder::default()).call()?;
    println!("fetch succeeded: {}", content);

    Ok(())
}
