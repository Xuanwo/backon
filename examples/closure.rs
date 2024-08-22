// For more examples, please see: https://docs.rs/backon/#examples

// this example does not run on wasm32-unknown-unknown
#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    use backon::BlockingRetryable;

    let var = 42;
    // `f` can use input variables
    let f = || Ok::<u32, anyhow::Error>(var);
    let result = f.retry(&backon::ExponentialBuilder::default()).call()?;
    println!("var = {result}");

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
