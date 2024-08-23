// For more examples, please see: https://docs.rs/backon/#examples

// this example does not run on wasm32-unknown-unknown
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use backon::Retryable;

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await?;

    let row: (i64,) = (|| sqlx::query_as("SELECT $1").bind(150_i64).fetch_one(&pool))
        .retry(backon::ExponentialBuilder::default())
        .await?;

    assert_eq!(row.0, 150);

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
