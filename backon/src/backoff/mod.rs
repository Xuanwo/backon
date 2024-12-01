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

trait Random {
    #[cfg(not(feature = "std"))]
    fn seed(&self) -> u64;

    #[cfg(not(feature = "std"))]
    fn set_seed(&mut self, seed: u64);

    fn jitter(&mut self) -> f32 {
        #[cfg(feature = "std")]
        return fastrand::f32();

        #[cfg(not(feature = "std"))]
        {
            let result = fastrand::Rng::with_seed(self.seed()).f32();
            // change the seed to get a new random number next time
            self.set_seed(self.seed() ^ result.to_bits() as u64);
            result
        }
    }
}
