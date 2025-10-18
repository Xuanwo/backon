use backon::{BackoffBuilder, Retryable};
use std::future::Future;
use tower::Service;

pub struct Retry<B: BackoffBuilder + Clone, S> {
    backoff: B,
    service: S,
}

impl<B: BackoffBuilder + Clone, S> Retry<B, S> {
    pub fn new(backoff: B, service: S) -> Self {
        Retry { backoff, service }
    }
}

impl<B, S, Request> Service<Request> for Retry<B, S>
where
    Request: Clone,
    B: BackoffBuilder + Clone,
    S: Service<Request> + Clone,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = impl Future<Output = Result<S::Response, S::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        // NOTE: the Future::poll impl for ResponseFuture assumes that Retry::poll_ready is
        // equivalent to Ready.service.poll_ready. If this ever changes, that code must be updated
        // as well.
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let req = request.clone();
        let mut srv = self.service.clone();
        let backoff = self.backoff.clone();

        (move || srv.call(req.clone())).retry(backoff.clone())
    }
}
