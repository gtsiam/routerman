use std::{pin::Pin, sync::Arc};

use futures_util::{Future, FutureExt};

use crate::response::IntoResponse;

pub(crate) type HandlerFuture<'h, Res> = Pin<Box<dyn Future<Output = Res> + Send + 'h>>;
pub(crate) type Handler<'h, Req, Res, Fmt> =
    dyn Fn(Req, Fmt) -> HandlerFuture<'h, Res> + Send + Sync + 'h;

#[derive(Clone)]
pub struct Route<'h, Req, Res, Fmt>(Arc<Handler<'h, Req, Res, Fmt>>);

impl<'h, Req, Res, Fmt> Route<'h, Req, Res, Fmt>
where
    Fmt: Send + Sync + 'h,
{
    pub fn new<H, Fut, Out>(handler: H) -> Self
    where
        H: Fn(Req) -> Fut + Send + Sync + 'h,
        Fut: Future<Output = Out> + Send + 'h,
        Out: IntoResponse<Res, Fmt>,
    {
        Self::with_fmt(move |req, fmt| handler(req).map(|res| res.into_response(fmt).0))
    }

    pub fn with_fmt<H, Fut>(handler: H) -> Self
    where
        H: Fn(Req, Fmt) -> Fut + Send + Sync + 'h,
        Fut: Future<Output = Res> + Send + 'h,
    {
        Self(Arc::new(move |req, fmt| Box::pin(handler(req, fmt))))
    }

    // pub fn call(&self, req: Req, fmt: Fmt) -> impl Future<Output = Res> + 'h {
    //     (self.0)(req, fmt)
    // }

    pub(crate) fn handler(&self) -> &Handler<'h, Req, Res, Fmt> {
        &*self.0
    }
}

impl<'h, H, Fut, Req, Res, Fmt, Out> From<H> for Route<'h, Req, Res, Fmt>
where
    H: Fn(Req) -> Fut + Send + Sync + 'h,
    Fut: Future<Output = Out> + Send + 'h,
    Out: IntoResponse<Res, Fmt>,
    Fmt: Send + Sync + 'h,
{
    fn from(handler: H) -> Self {
        Self::new(handler)
    }
}
