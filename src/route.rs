use std::{pin::Pin, sync::Arc};

use futures_util::{Future, FutureExt};

use crate::{response::IntoResponse, Request, Response};

pub(crate) type HandlerFuture = Pin<Box<dyn Future<Output = Response> + Send>>;
pub(crate) type Handler = dyn Fn(Request) -> HandlerFuture + Send + Sync;

#[derive(Clone)]
pub struct Route(Arc<Handler>);

impl Route {
    pub fn new<H, F, R>(handler: H) -> Self
    where
        H: Fn(Request) -> F + Send + Sync + 'static,
        F: Future<Output = R> + Send + 'static,
        R: IntoResponse,
    {
        Self(Arc::new(move |req| {
            Box::pin(handler(req).map(|res| res.into_response()))
        }))
    }

    pub fn call(&self, req: Request) -> impl Future<Output = Response> {
        (self.0)(req)
    }

    pub(crate) fn handler(&self) -> &Handler {
        &*self.0
    }
}

impl<H, F, R> From<H> for Route
where
    H: Fn(Request) -> F + Send + Sync + 'static,
    F: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    fn from(handler: H) -> Self {
        Self::new(handler)
    }
}
