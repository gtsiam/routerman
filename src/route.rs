use crate::{
    request::Request,
    response::{Reply, Response},
};
use futures_util::{Future, FutureExt};
use std::{pin::Pin, sync::Arc};

pub(crate) type BoxFuture<Out> = Pin<Box<dyn Future<Output = Out> + Send + 'static>>;
type HandlerFn<Fmt> = dyn Fn(Request, Fmt) -> BoxFuture<Response> + Send + Sync + 'static;

#[derive(Clone)]
pub struct Route<Fmt>(Arc<HandlerFn<Fmt>>);

impl<Fmt> Route<Fmt> {
    pub fn new<H, Args>(handler: H) -> Self
    where
        H: RouteHandler<Fmt, Args>,
        Fmt: Send + Sync + 'static,
    {
        handler.into_route()
    }

    pub(crate) fn handler_fn(&self) -> &HandlerFn<Fmt> {
        &*self.0
    }
}

/// Route handler. Implemened on any type that can be meaningfully converted into a route.
///
/// Note: The Args type argument is there to allow implementing on conflicting types (eg. `Fn(T1)`
/// and `Fn(T1, T2)` which can, in theory, be implemented on the same type).
pub trait RouteHandler<Fmt, Args> {
    fn into_route(self) -> Route<Fmt>;
}

/// impl Handler for `async Fn(Req) -> Out`
impl<H, Fut, Fmt, Out> RouteHandler<Fmt, (Request,)> for H
where
    H: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Out> + Send + 'static,
    Out: Reply<Fmt>,
    Fmt: Send + Sync + 'static,
{
    fn into_route(self) -> Route<Fmt> {
        (move |req, fmt| self(req).map(|res| res.reply(fmt))).into_route()
    }
}

/// impl Handler for `async Fn(Req, Fmt) -> Res`
impl<H, Fut, Fmt> RouteHandler<Fmt, (Request, Fmt)> for H
where
    H: Fn(Request, Fmt) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
    Fmt: Send + Sync + 'static,
{
    fn into_route(self) -> Route<Fmt> {
        Route(Arc::new(move |req, fmt| Box::pin(self(req, fmt))))
    }
}
