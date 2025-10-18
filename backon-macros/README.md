# backon-macros &emsp; [![Build Status]][actions] [![Latest Version]][crates.io]

[Build Status]: https://img.shields.io/github/actions/workflow/status/Xuanwo/backon/ci.yml?branch=main
[actions]: https://github.com/Xuanwo/backon/actions?query=branch%3Amain
[Latest Version]: https://img.shields.io/crates/v/backon-macros.svg
[crates.io]: https://crates.io/crates/backon-macros

Attribute macros that make [`backon`](https://crates.io/crates/backon) retry APIs feel like native syntax.

- **Fluent ergonomics**: Mark a function with `#[backon]` and reuse the builder-style retry API without writing closures.
- **Async & sync aware**: The macro inspects your signature to choose between `Retryable` and `BlockingRetryable`.
- **Configurable**: Opt into custom backoff strategies, sleepers, filters, notifications, or context capture with simple arguments.

---

## Installation

Add both `backon` and `backon-macros` to your `Cargo.toml`:

```toml
[dependencies]
backon = "1"
backon-macros = "1"
```

## Usage

Annotate any free function or inherent method (taking `&self`) with `#[backon]`. The macro rewrites the body so it runs inside a retry loop using the `backon` primitives.

```rust
use anyhow::Result;
use backon::TokioSleeper;
use backon_macros::backon;

#[backon(
    backoff = backon::ExponentialBuilder::default,
    sleep = TokioSleeper::sleep,
    when = |e: &anyhow::Error| e.to_string() == "temporary",
    notify = |err: &anyhow::Error, dur| tracing::warn!(?err, ?dur, "retrying")
)]
async fn fetch_data() -> Result<String> {
    Ok("value".to_string())
}
```

### Parameters

| Argument | Description |
| --- | --- |
| `backoff = path` | Builder function that returns the backoff strategy. Defaults to `backon::ExponentialBuilder::default`. |
| `sleep = path` | Sleeper implementation for async or blocking retries. |
| `when = path` | Predicate used to decide if an error should trigger another attempt. |
| `notify = path` | Callback invoked before sleeping. |
| `adjust = path` | Async-only hook that can modify the next delay. |
| `context = true` | Capture arguments into a context tuple for `RetryableWithContext`. Use when the closure would otherwise borrow values that cannot cross await points. |

### Context mode

`context = true` is available for free functions and methods without receivers, and for methods that take ownership of their receiver. It cannot be combined with `&self` or `&mut self` methods; those cases must still use manual `RetryableWithContext` until dedicated support is added.

## Examples

Try the ready-to-run programs under [`examples/`](./examples) with `cargo run --example ...` for async, blocking, and method-based workflows. For more exhaustive coverage, check the [trybuild tests](./tests/cases).

## Limitations

- Methods that take `&mut self` or own `self` are currently rejected with a compile-time error; write manual retry loops for now.
- The macro does not support arbitrary patterns in parameter position—each argument must bind to an identifier.
- The generated code depends on the `backon` crate at runtime; ensure both crates are included in your workspace.

## License

Licensed under <a href="../LICENSE">Apache License, Version 2.0</a>.
