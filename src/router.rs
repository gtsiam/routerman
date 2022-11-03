use core::fmt;
use std::{
    convert::Infallible,
    future::{Future, Ready},
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_util::ready;
use hyper::{server::conn::AddrStream, Body};
use matchit::MatchError;
use pin_project::pin_project;
use thiserror::Error;
use tower_service::Service;

use crate::{
    request::{
        ext::{InvalidParamEncoding, RemoteAddrExt, RouteParamsExt},
        Request,
    },
    response::{DefaultFormatter, Reply, Response},
    route::{BoxFuture, Route, RouteHandler},
};

pub struct Router<Fmt = DefaultFormatter> {
    formatter: Fmt,
    inner: Arc<RouterImpl<Fmt>>,
}

struct RouterImpl<Fmt> {
    inner: matchit::Router<Route<Fmt>>,
    default: Option<Route<Fmt>>,
}

impl<Fmt> Router<Fmt> {
    pub fn builder() -> RouterBuilder<Fmt> {
        RouterBuilder {
            routes: Vec::new(),
            default: None,
        }
    }
}

pub struct RouterBuilder<Fmt = DefaultFormatter> {
    routes: Vec<(String, Route<Fmt>)>,
    default: Option<Route<Fmt>>,
}

impl<Fmt> RouterBuilder<Fmt>
where
    Fmt: Clone + Send + Sync + 'static,
{
    pub fn route<P, H, Args>(mut self, path: P, handler: H) -> Self
    where
        P: Into<String>,
        H: RouteHandler<Fmt, Args>,
    {
        self.routes.push((path.into(), handler.into_route()));
        self
    }

    pub fn default_route<H, Args>(mut self, route: H) -> Self
    where
        H: RouteHandler<Fmt, Args>,
    {
        self.default = Some(route.into_route());
        self
    }

    pub fn merge(self, router: RouterBuilder<Fmt>) -> Self {
        let Self {
            mut routes,
            mut default,
        } = self;

        // Record all the new routes
        for (path, route) in router.routes {
            routes.push((path, route));
        }

        // Merge default routes
        if let Some(route) = router.default {
            if default.replace(route).is_some() {
                panic!("cannot merge routers with conflicting default routes")
            }
        }

        Self { routes, default }
    }

    pub fn build(self) -> Router<Fmt>
    where
        Fmt: Default,
    {
        let mut inner = matchit::Router::new();
        for (path, route) in self.routes.into_iter() {
            inner.insert(path, route).expect("insert route");
        }

        Router {
            inner: Arc::new(RouterImpl {
                inner,
                default: self.default,
            }),
            formatter: Fmt::default(), // TODO: Non-default formatter
        }
    }
}

impl<Fmt> Service<&AddrStream> for Router<Fmt>
where
    Fmt: Clone,
{
    type Response = RequestService<Fmt>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, conn: &AddrStream) -> Self::Future {
        let remote_addr = conn.remote_addr();
        std::future::ready(Ok(RequestService {
            remote_addr,
            router: self.inner.clone(),
            formatter: self.formatter.clone(),
        }))
    }
}

pub struct RequestService<Fmt> {
    formatter: Fmt,
    remote_addr: SocketAddr,
    router: Arc<RouterImpl<Fmt>>,
}

#[derive(Error)]
#[error("{kind}")]
pub struct RouteError {
    pub request: Request,

    #[source]
    pub kind: RouteErrorKind,
}

impl fmt::Debug for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouteError")
            .field("kind", &self.kind)
            .finish()
    }
}

#[derive(Debug, Error)]
pub enum RouteErrorKind {
    /// No matching route was found
    #[error("not found")]
    NotFound,

    #[error("extra trailing slash")]
    ExtraTrailingSlash,

    #[error("missing trailing slash")]
    MissingTrailingSlash,

    /// There was an error decoding the uri path
    #[error("invalid param encoding: {0}")]
    Param(InvalidParamEncoding),
}

impl<Fmt> Service<hyper::Request<Body>> for RequestService<Fmt>
where
    RouteError: Reply<Fmt>,
    Fmt: Clone,
{
    type Response = Response;
    type Error = Infallible;
    type Future = RequestFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // It's not possible to know if the route in question is ready, because the request has not
        // been received yet. Meaning that backpressure across the router boundry is not possible.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: hyper::Request<Body>) -> Self::Future {
        // Add connection address information to the request's extensions
        req.extensions_mut().insert(RemoteAddrExt(self.remote_addr));

        let res = match self.router.inner.at(req.uri().path()) {
            // A route was found. Attempt to parse the parameters and run the handler. If the
            // parameters are invalid (eg. invalid percent-encoded utf8), reply with error.
            Ok(route) => match RouteParamsExt::try_from(route.params) {
                Ok(params) => Ok((route.value, Some(params))),
                Err(err) => Err(RouteErrorKind::Param(err)),
            },
            // No route was found. Use the fallback if it exists, otherwise reply with error.
            Err(MatchError::NotFound) => match self.router.default {
                Some(ref route) => Ok((route, None)),
                None => Err(RouteErrorKind::NotFound),
            },

            // There was either a trailing slash when there shouldn't be, or there wasn't a trailing
            // slash when there should be. Reply with an error that Indicates to redirect the user
            // to the correct path.
            Err(MatchError::ExtraTrailingSlash) => Err(RouteErrorKind::ExtraTrailingSlash),
            Err(MatchError::MissingTrailingSlash) => Err(RouteErrorKind::MissingTrailingSlash),
        };

        // Finally return the request future, either containing the route's future or an immediate
        // reponse
        match res {
            Ok((route, params)) => {
                if let Some(params) = params {
                    req.extensions_mut().insert(params);
                }

                RequestFuture::Route((route.handler_fn())(req, self.formatter.clone()))
            }
            Err(err) => RequestFuture::Response(Some(
                RouteError {
                    request: req,
                    kind: err,
                }
                .reply(self.formatter.clone()),
            )),
        }
    }
}

#[pin_project(project = RequestFutureProj)]
pub enum RequestFuture {
    Route(#[pin] BoxFuture<Response>),
    Response(Option<Response>),
}

impl Future for RequestFuture {
    type Output = Result<Response, Infallible>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().project() {
            RequestFutureProj::Route(fut) => Poll::Ready(Ok(ready!(fut.poll(cx)))),
            RequestFutureProj::Response(res @ Some(_)) => Poll::Ready(Ok(res.take().unwrap())),
            RequestFutureProj::Response(None) => panic!("future polled after completion"),
        }
    }
}
