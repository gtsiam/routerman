use futures_util::Future;
use hyper::{
    body::{Bytes, HttpBody},
    Request,
};
use std::pin::Pin;

pub trait ExtractFrom<Req: ?Sized>: Sized {
    type Error;
    type Future: Future<Output = Result<Self, Self::Error>>;

    fn extract_from(req: Req) -> Self::Future;
}

impl<B> ExtractFrom<Request<B>> for Bytes
where
    B: HttpBody + Send + 'static,
    B::Data: Send,
{
    type Error = B::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>> + Send>>;

    fn extract_from(req: Request<B>) -> Self::Future {
        Box::pin(async move { hyper::body::to_bytes(req.into_body()).await })
    }
}
