use axum::http::{HeaderName, HeaderValue, Request, Response};
use std::task::{Context, Poll};
use tower::{Layer, Service};
use uuid::Uuid;

static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Clone)]
pub struct RequestIdLayer;

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdService { inner }
    }
}

#[derive(Clone)]
pub struct RequestIdService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for RequestIdService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = RequestIdFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let request_id = Uuid::new_v4().to_string();
        req.extensions_mut().insert(RequestId(request_id.clone()));

        RequestIdFuture {
            future: self.inner.call(req),
            request_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestId(pub String);

pin_project_lite::pin_project! {
    pub struct RequestIdFuture<F> {
        #[pin]
        future: F,
        request_id: String,
    }
}

impl<F, ResBody, E> std::future::Future for RequestIdFuture<F>
where
    F: std::future::Future<Output = Result<Response<ResBody>, E>>,
{
    type Output = Result<Response<ResBody>, E>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.future.poll(cx) {
            Poll::Ready(Ok(mut response)) => {
                if let Ok(value) = HeaderValue::from_str(this.request_id) {
                    response.headers_mut().insert(X_REQUEST_ID.clone(), value);
                }
                Poll::Ready(Ok(response))
            }
            other => other,
        }
    }
}
