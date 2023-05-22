use anyhow::Result;
use backon::ExponentialBuilder;
use backon::Retryable;

async fn fetch() -> Result<String> {
    let response = reqwest::get("https://httpbingo.org/unstable?failure_rate=0.7").await?;
    if !response.status().is_success() {
        println!("{}", response.status());
        anyhow::bail!("some kind of error");
    }
    let text = response.text().await?;
    Ok(text)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let _ = fetch.retry(&ExponentialBuilder::default()).await?;
    println!("fetch succeeded");

    Ok(())
}
