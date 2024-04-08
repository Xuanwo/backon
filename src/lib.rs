//! backon intends to provide an opposite backoff implementation of the popular [backoff](https://docs.rs/backoff).
//!
//! - Newer: developed by Rust edition 2021 and latest stable.
//! - Cleaner: Iterator based abstraction, easy to use, customization friendly.
//! - Easier: Trait based implementations, works like a native function provided by closures.
//!
//! # Backoff
//!
//! Any types that implements `Iterator<Item = Duration>` can be used as backoff.
//!
//! backon also provides backoff implementations with reasonable defaults:
//!
//! - [`ConstantBackoff`]: backoff with constant delay and limited times.
//! - [`ExponentialBackoff`]: backoff with exponential delay, also provides jitter supports.
//! - [`FibonacciBackoff`]: backoff with fibonacci delay, also provides jitter supports.
//!
//! Internally, `tokio::time::sleep()` will be used to sleep between retries, therefore
//! it will respect [pausing/auto-advancing](https://docs.rs/tokio/latest/tokio/time/fn.pause.html)
//! tokio's Runtime semantics, if enabled.
//!
//! # Examples
//!
//! Retry with default settings.
//!
//! ```no_run
//! use anyhow::Result;
//! use backon::ExponentialBuilder;
//! use backon::Retryable;
//!
//! async fn fetch() -> Result<String> {
//!     Ok(reqwest::get("https://www.rust-lang.org")
//!         .await?
//!         .text()
//!         .await?)
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<()> {
//!     let content = fetch.retry(&ExponentialBuilder::default()).await?;
//!
//!     println!("fetch succeeded: {}", content);
//!     Ok(())
//! }
//! ```
//!
//! Retry with specify retryable error.
//!
//! ```no_run
//! use anyhow::Result;
//! use backon::ExponentialBuilder;
//! use backon::Retryable;
//!
//! async fn fetch() -> Result<String> {
//!     Ok(reqwest::get("https://www.rust-lang.org")
//!         .await?
//!         .text()
//!         .await?)
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<()> {
//!     let content = fetch
//!         .retry(&ExponentialBuilder::default())
//!         .when(|e| e.to_string() == "retryable")
//!         .await?;
//!
//!     println!("fetch succeeded: {}", content);
//!     Ok(())
//! }
//! ```
//!
//! Retry functions with args.
//!
//! ```no_run
//! use anyhow::Result;
//! use backon::ExponentialBuilder;
//! use backon::Retryable;
//!
//! async fn fetch(url: &str) -> Result<String> {
//!     Ok(reqwest::get(url).await?.text().await?)
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<()> {
//!     let content = (|| async { fetch("https://www.rust-lang.org").await })
//!         .retry(&ExponentialBuilder::default())
//!         .when(|e| e.to_string() == "retryable")
//!         .await?;
//!
//!     println!("fetch succeeded: {}", content);
//!     Ok(())
//! }
//! ```
//!
//! Retry functions with receiver `&self`.
//!
//! ```no_run
//! use anyhow::Result;
//! use backon::ExponentialBuilder;
//! use backon::Retryable;
//!
//! struct Test;
//!
//! impl Test {
//!     async fn fetch(&self, url: &str) -> Result<String> {
//!         Ok(reqwest::get(url).await?.text().await?)
//!     }
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<()> {
//!     let test = Test;
//!     let content = (|| async { test.fetch("https://www.rust-lang.org").await })
//!         .retry(&ExponentialBuilder::default())
//!         .when(|e| e.to_string() == "retryable")
//!         .await?;
//!
//!     println!("fetch succeeded: {}", content);
//!     Ok(())
//! }
//! ```
//!
//! Retry functions with receiver `&mut self`.
//!
//! ```no_run
//! use anyhow::Result;
//! use backon::ExponentialBuilder;
//! use backon::RetryableWithContext;
//!
//! struct Test;
//!
//! impl Test {
//!     async fn fetch(&mut self, url: &str) -> Result<String> {
//!         Ok(reqwest::get(url).await?.text().await?)
//!     }
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<()> {
//!     let test = Test;
//!
//!     let (_, result) = (|mut v: Test| async {
//!         let res = v.fetch("https://www.rust-lang.org").await;
//!         // Return input context back.
//!         (v, res)
//!     })
//!     .retry(&ExponentialBuilder::default())
//!     // Passing context in.
//!     .context(test)
//!     .when(|e| e.to_string() == "retryable")
//!     .await;
//!
//!     println!("fetch succeeded: {}", result.unwrap());
//!     Ok(())
//! }
//! ```

#![deny(missing_docs)]
#![deny(unused_qualifications)]

mod backoff;
pub use backoff::Backoff;
pub use backoff::BackoffBuilder;

mod constant;
pub use constant::ConstantBackoff;
pub use constant::ConstantBuilder;

mod exponential;
pub use exponential::ExponentialBackoff;
pub use exponential::ExponentialBuilder;

mod fibonacci;
pub use fibonacci::FibonacciBackoff;
pub use fibonacci::FibonacciBuilder;

mod retry;
pub use retry::Retry;
pub use retry::Retryable;

mod retry_with_context;
pub use retry_with_context::RetryableWithContext;

mod blocking_retry;
pub use blocking_retry::BlockingRetry;
pub use blocking_retry::BlockingRetryable;

mod blocking_retry_with_context;
pub use blocking_retry_with_context::BlockingRetryableWithContext;
