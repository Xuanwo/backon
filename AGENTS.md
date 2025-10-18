# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Project Overview

BackON is a Rust retry library that provides a fluent API for implementing retry logic with various backoff strategies. The library supports both synchronous and asynchronous operations, with cross-platform support including WASM and no_std environments.

## Core Architecture

- **Backoff Strategies** (`backon/src/backoff/`): Different retry timing strategies
  - `ExponentialBackoff`: Exponential backoff with optional jitter
  - `ConstantBackoff`: Fixed delay between retries
  - `FibonacciBackoff`: Fibonacci sequence-based delays
  - All implement the `Backoff` trait defined in `api.rs`

- **Sleep Implementations** (`backon/src/sleep.rs`, `backon/src/blocking_sleep.rs`): Platform-specific sleep implementations
  - Async sleepers: `TokioSleeper`, `GlooTimersSleep`, `FutureTimerSleep`, `EmbassySleeper`
  - Blocking sleepers: `StdSleeper`
  - All sleepers implement `Sleeper` or `BlockingSleeper` traits

- **Retry Logic** (`backon/src/retry.rs`, `backon/src/blocking_retry.rs`): Core retry functionality
  - `Retryable` trait for async functions
  - `BlockingRetryable` trait for sync functions
  - `RetryableWithContext` for functions that need mutable context

- **Features**: Conditional compilation based on target and enabled features
  - Default features enable common sleepers for different platforms
  - WASM-specific implementations using `gloo-timers`
  - Embassy support for embedded environments

## Development Commands

```bash
# Check code for errors
cargo check

# Build the project
cargo build

# Run linting (clippy)
cargo clippy

# Run tests
cargo test

# Run benchmarks
cargo bench

# Format code
cargo fmt
```

## Testing

- Unit tests are in each module alongside the implementation
- Integration tests demonstrate real-world usage patterns
- WASM tests use `wasm-bindgen-test`
- Platform-specific tests are gated by target architecture

## Key Patterns

- Builder pattern for backoff strategies (e.g., `ExponentialBuilder::default()`)
- Trait-based design for extensibility (`Backoff`, `Sleeper`, `Retryable`)
- Feature-gated implementations for different platforms
- Zero-cost abstractions with compile-time feature selection
- No-std compatibility with optional std features
