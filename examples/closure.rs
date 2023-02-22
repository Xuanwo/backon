use anyhow::Result;
use backon::BlockingRetryable;
use backon::ExponentialBuilder;

fn main() -> Result<()> {
    let var = 42;
    // `f` can use input variables
    let f = || Ok::<u32, anyhow::Error>(var);
    let result = f.retry(&ExponentialBuilder::default()).call()?;
    println!("var = {result}");

    Ok(())
}
