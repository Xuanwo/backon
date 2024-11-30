mod api;
pub use api::*;

mod constant;
pub use constant::ConstantBackoff;
pub use constant::ConstantBuilder;

mod fibonacci;
pub use fibonacci::FibonacciBackoff;
pub use fibonacci::FibonacciBuilder;

mod exponential;
pub use exponential::ExponentialBackoff;
pub use exponential::ExponentialBuilder;

#[cfg(feature = "std")]
fn f32() -> f32 {
    fastrand::f32()
}

#[cfg(all(not(feature = "std"), feature = "embassy-time"))]
fn f32() -> f32 {
    fastrand::Rng::with_seed(embassy_time::Instant::now().as_micros()).f32()
}

#[cfg(all(not(feature = "std"), not(feature = "embassy-time")))]
fn f32() -> f32 {
    fastrand::Rng::with_seed(0x2fdb0020ffc7722b).f32()
}
