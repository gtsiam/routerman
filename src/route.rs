use std::{pin::Pin, sync::Arc};

use futures_util::{Future, FutureExt};

use crate::{response::IntoResponse, Request, Response};

pub(crate) type HandlerFuture = Pin<Box<dyn Future<Output = Response> + Send>>;
pub(crate) type Handler<Fmt> = dyn Fn(Request, Fmt) -> HandlerFuture + Send + Sync;

#[derive(Clone)]
pub struct Route<Fmt>(Arc<Handler<Fmt>>);

impl<Fmt> Route<Fmt>
where
    Fmt: Send + Sync + 'static,
{
    pub fn new<H, Fut, Res>(handler: H) -> Self
    where
        H: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Res> + Send + 'static,
        Res: IntoResponse<Fmt>,
    {
        Self(Arc::new(move |req, fmt| {
            Box::pin(handler(req).map(|res| res.into_response(fmt)))
        }))
    }

    pub fn with_fmt<H, Fut, Res>(handler: H) -> Self
    where
        H: Fn(Request, Fmt) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Res> + Send + 'static,
        Res: IntoResponse<Fmt>,
        Fmt: Clone,
    {
        Self(Arc::new(move |req, fmt| {
            Box::pin(handler(req, fmt.clone()).map(|res| res.into_response(fmt)))
        }))
    }

    pub fn call(&self, req: Request, fmt: Fmt) -> impl Future<Output = Response> {
        (self.0)(req, fmt)
    }

    pub(crate) fn handler(&self) -> &Handler<Fmt> {
        &*self.0
    }
}

pub trait Func<Args> {
    type Output;

    fn call(&self, args: Args) -> Self::Output;
}

impl<H, Fut, Res, Fmt> From<H> for Route<Fmt>
where
    H: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse<Fmt>,
    Fmt: Send + Sync + 'static,
{
    fn from(handler: H) -> Self {
        Self::new(handler)
    }
}

// impl<H, Fut, Res, Fmt> From<H> for Route<Fmt>
// where
//     H: Fn(Request, &Fmt) -> Fut + Send + Sync + 'static,
//     Fut: Future<Output = Res> + Send + 'static,
//     Res: IntoResponse<Fmt>,
//     Fmt: Send + Sync + 'static,
// {
//     fn from(handler: H) -> Self {
//         Self::with_fmt(handler)
//     }
// }
