use std::{pin::Pin, sync::Arc};

use futures_util::{Future, FutureExt};

use crate::response::IntoResponse;

pub(crate) type HandlerFuture<'h, Res> = Pin<Box<dyn Future<Output = Res> + Send + 'h>>;
pub(crate) type HandlerFn<'h, Req, Res, Fmt> =
    dyn Fn(Req, Fmt) -> HandlerFuture<'h, Res> + Send + Sync + 'h;

#[derive(Clone)]
pub struct Route<'h, Req, Res, Fmt>(Arc<HandlerFn<'h, Req, Res, Fmt>>);

impl<'h, Req, Res, Fmt> Route<'h, Req, Res, Fmt>
where
    Fmt: Send + Sync + 'h,
{
    pub fn new<H, Args>(handler: H) -> Self
    where
        H: RouteHandler<'h, Req, Res, Fmt, Args>,
    {
        handler.into_route()
    }

    pub(crate) fn handler_fn(&self) -> &HandlerFn<'h, Req, Res, Fmt> {
        &*self.0
    }
}

/// Route handler. Implemened on any type that can be meaningfully converted into a route.
///
/// Note: The Args type argument is there to allow implementing on conflicting types (eg. `Fn(T1)`
/// and `Fn(T1, T2)` which can, in theory, be implemented on the same type).
pub trait RouteHandler<'h, Req, Res, Fmt, Args> {
    fn into_route(self) -> Route<'h, Req, Res, Fmt>;
}

/// impl Handler for `async Fn(Req) -> Out`
impl<'h, H, Fut, Req, Res, Fmt, Out> RouteHandler<'h, Req, Res, Fmt, (Req,)> for H
where
    H: Fn(Req) -> Fut + Send + Sync + 'h,
    Fut: Future<Output = Out> + Send + 'h,
    Out: IntoResponse<Res, Fmt>,
    Fmt: Send + Sync + 'h,
{
    fn into_route(self) -> Route<'h, Req, Res, Fmt> {
        (move |req, fmt| self(req).map(|res| res.into_response(fmt).0)).into_route()
    }
}

/// impl Handler for `async Fn(Req, Fmt) -> Res`
impl<'h, H, Fut, Req, Res, Fmt> RouteHandler<'h, Req, Res, Fmt, (Req, Fmt)> for H
where
    H: Fn(Req, Fmt) -> Fut + Send + Sync + 'h,
    Fut: Future<Output = Res> + Send + 'h,
    Fmt: Send + Sync + 'h,
{
    fn into_route(self) -> Route<'h, Req, Res, Fmt> {
        Route(Arc::new(move |req, fmt| Box::pin(self(req, fmt))))
    }
}
