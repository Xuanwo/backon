#![feature(impl_trait_in_assoc_type)]

mod layer;
pub use layer::RetryLayer;
mod retry;
pub use self::retry::Retry;
