use std::{pin::Pin, sync::Arc};

use futures_util::{Future, FutureExt};

use crate::{response::IntoResponse, Request, Response};

pub(crate) type HandlerFuture<'h> = Pin<Box<dyn Future<Output = Response> + Send + 'h>>;
pub(crate) type Handler<'h, Fmt> = dyn Fn(Request, Fmt) -> HandlerFuture<'h> + Send + Sync + 'h;

#[derive(Clone)]
pub struct Route<'h, Fmt>(Arc<Handler<'h, Fmt>>);

impl<'h, Fmt> Route<'h, Fmt>
where
    Fmt: Send + Sync + 'h,
{
    pub fn new<H, Fut, Res>(handler: H) -> Self
    where
        H: Fn(Request) -> Fut + Send + Sync + 'h,
        Fut: Future<Output = Res> + Send + 'h,
        Res: IntoResponse<Fmt>,
    {
        Self(Arc::new(move |req, fmt| {
            Box::pin(handler(req).map(|res| res.into_response(fmt)))
        }))
    }

    pub fn with_fmt<H, Fut, Res>(handler: H) -> Self
    where
        H: Fn(Request, Fmt) -> Fut + Send + Sync + 'h,
        Fut: Future<Output = Res> + Send + 'h,
        Res: IntoResponse<Fmt>,
        Fmt: Clone,
    {
        Self(Arc::new(move |req, fmt| {
            Box::pin(handler(req, fmt.clone()).map(|res| res.into_response(fmt)))
        }))
    }

    pub fn call(&self, req: Request, fmt: Fmt) -> impl Future<Output = Response> + 'h {
        (self.0)(req, fmt)
    }

    pub(crate) fn handler(&self) -> &Handler<'h, Fmt> {
        &*self.0
    }
}

impl<'h, H, Fut, Res, Fmt> From<H> for Route<'h, Fmt>
where
    H: Fn(Request) -> Fut + Send + Sync + 'h,
    Fut: Future<Output = Res> + Send + 'h,
    Res: IntoResponse<Fmt>,
    Fmt: Send + Sync + 'h,
{
    fn from(handler: H) -> Self {
        Self::new(handler)
    }
}
