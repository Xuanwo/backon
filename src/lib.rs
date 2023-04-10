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
//! #[tokio::main]
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
//! #[tokio::main]
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

mod blocking_retry;
pub use blocking_retry::BlockingRetry;
pub use blocking_retry::BlockingRetryable;
