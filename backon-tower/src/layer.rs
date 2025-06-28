use crate::Retry;
use backon::BackoffBuilder;
use tower::Layer;

pub struct RetryLayer<B: BackoffBuilder + Clone> {
    backoff: B,
}

impl<B, S> Layer<S> for RetryLayer<B>
where
    B: BackoffBuilder + Clone,
{
    type Service = Retry<B, S>;

    fn layer(&self, service: S) -> Self::Service {
        Retry::new(self.backoff.clone(), service)
    }
}
