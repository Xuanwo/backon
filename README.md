# backon &emsp; [![Build Status]][actions] [![Latest Version]][crates.io] [![](https://img.shields.io/discord/1111711408875393035?logo=discord&label=discord)](https://discord.gg/8ARnvtJePD)

[Build Status]: https://img.shields.io/github/actions/workflow/status/Xuanwo/backon/ci.yml?branch=main

[actions]: https://github.com/Xuanwo/backon/actions?query=branch%3Amain

[Latest Version]: https://img.shields.io/crates/v/backon.svg

[crates.io]: https://crates.io/crates/backon

Retry with backoff without effort.

---

The opposite backoff implementation of the popular [backoff](https://docs.rs/backoff).

- Newer: developed by Rust edition 2021 and latest stable.
- Cleaner: Iterator based abstraction, easy to use, customization friendly.
- Easier: Trait based implementations, works like a native function provided by closures.

## Quick Start

Retry a blocking function.

```rust
use anyhow::Result;
use backon::BlockingRetryable;
use backon::ExponentialBuilder;

fn fetch() -> Result<String> {
     Ok("hello, world!".to_string())
}

fn main() -> Result<()> {
    let content = fetch.retry(&ExponentialBuilder::default()).call()?;
    println!("fetch succeeded: {}", content);

    Ok(())
}
```

Retry an async function.

```rust
use anyhow::Result;
use backon::ExponentialBuilder;
use backon::Retryable;

async fn fetch() -> Result<String> {
    Ok(reqwest::get("https://www.rust-lang.org").await?.text().await?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let content = fetch.retry(&ExponentialBuilder::default()).await?;
    println!("fetch succeeded: {}", content);

    Ok(())
}
```

## Contributing

Check out the [CONTRIBUTING.md](./CONTRIBUTING.md) guide for more details on getting started with contributing to this
project.

## Getting help

Submit [issues](https://github.com/Xuanwo/backon/issues/new/choose) for bug report or asking questions
in [discussion](https://github.com/Xuanwo/backon/discussions/new?category=q-a).

#### License

<sup>
Licensed under <a href="./LICENSE">Apache License, Version 2.0</a>.
</sup>
