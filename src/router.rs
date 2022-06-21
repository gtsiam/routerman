use core::fmt::Display;
use std::{
    convert::Infallible,
    future::{Future, Ready},
    marker::PhantomData,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_util::ready;
use hyper::{header::HeaderValue, server::conn::AddrStream, Body, Request, Response, Uri};
use matchit::MatchError;
use pin_project::pin_project;
use thiserror::Error;
use tower_service::Service;

use crate::{
    request::ext::{InvalidParamEncoding, RemoteAddrExt, RouteParamsExt},
    response::{DefaultFormatter, Formatter},
    route::{HandlerFuture, Route, RouteHandler},
};

pub struct Router<'h, Req, Res, Fmt = DefaultFormatter> {
    formatter: Fmt,
    inner: Arc<RouterImpl<'h, Req, Res, Fmt>>,
}

struct RouterImpl<'h, Req, Res, Fmt> {
    inner: matchit::Router<Route<'h, Req, Res, Fmt>>,
    default: Option<Route<'h, Req, Res, Fmt>>,
}

impl<'h, Req, Res, Fmt> Router<'h, Req, Res, Fmt> {
    pub fn builder() -> RouterBuilder<'h, Req, Res, Fmt> {
        RouterBuilder {
            routes: Vec::new(),
            default: None,
            _phantom: PhantomData,
        }
    }
}

pub struct RouterBuilder<'h, Req, Res, Fmt> {
    routes: Vec<(String, Route<'h, Req, Res, Fmt>)>,
    default: Option<Route<'h, Req, Res, Fmt>>,
    _phantom: PhantomData<Fmt>,
}

impl<'h, Req, Res, Fmt> RouterBuilder<'h, Req, Res, Fmt>
where
    Req: Send + 'static,
    Fmt: Clone + Send + Sync + 'h,
{
    pub fn route<P, H, Args>(mut self, path: P, handler: H) -> Self
    where
        P: Into<String>,
        H: RouteHandler<'h, Req, Res, Fmt, Args>,
    {
        self.routes.push((path.into(), handler.into_route()));
        self
    }

    pub fn default_route<H, Args>(mut self, route: H) -> Self
    where
        H: RouteHandler<'h, Req, Res, Fmt, Args>,
    {
        self.default = Some(route.into_route());
        self
    }

    pub fn merge(self, router: RouterBuilder<'h, Req, Res, Fmt>) -> Self {
        let Self {
            mut routes,
            mut default,
            _phantom,
        } = self;

        // Record all the new routes
        for (path, route) in router.routes {
            routes.push((path, route));
        }

        // Merge default routes
        if let Some(route) = router.default {
            if let Some(_) = default.replace(route) {
                panic!("cannot merge routers with conflicting default routes")
            }
        }

        Self {
            routes,
            default,
            _phantom,
        }
    }

    pub fn build(self) -> Router<'h, Req, Res, Fmt>
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

impl<'h, Req, Res, Fmt> Service<&AddrStream> for Router<'h, Req, Res, Fmt>
where
    Fmt: Clone,
{
    type Response = RequestService<'h, Req, Res, Fmt>;
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

pub struct RequestService<'h, Req, Res, Fmt> {
    formatter: Fmt,
    remote_addr: SocketAddr,
    router: Arc<RouterImpl<'h, Req, Res, Fmt>>,
}

#[derive(Debug, Error)]
pub enum RouteError<'a> {
    /// No matching route was found
    #[error("not found")]
    NotFound,

    /// An almost matching route was found
    #[error("expected uri: {0}")]
    Expected(&'a Uri),

    /// There was an error decoding the uri path
    #[error("invalid param encoding: {0}")]
    Path(InvalidParamEncoding),

    /// The request method is not allowed
    #[error("method not allowed")]
    MethodNotAllowed { allow_header: &'a HeaderValue },
}

impl<'h, Fmt, B> Service<Request<B>> for RequestService<'h, Request<B>, Response<Body>, Fmt>
where
    Fmt: Clone + Send + Sync + 'h,
    Fmt: for<'a> Formatter<Response<Body>, RouteError<'a>>,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = RequestFuture<Self::Response, HandlerFuture<'h, Self::Response>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // It's not possible to know if the route in question is ready, because the request has not
        // been received yet. Meaning that backpressure across the router boundry is not possible.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        // Replace the path portion of a uri
        fn replace_path(uri: &Uri, path: impl Display) -> Uri {
            let mut parts = uri.to_owned().into_parts();
            parts.path_and_query = parts.path_and_query.map(|pq| {
                match pq.query() {
                    Some(query) => format!("{}?{}", path, query),
                    None => format!("{}", path),
                }
                .parse()
                .unwrap()
            });
            Uri::from_parts(parts).unwrap()
        }

        // Add connection address information to the request's extensions
        req.extensions_mut()
            .insert(RemoteAddrExt::from(self.remote_addr));

        let uri;
        let res = match self.router.inner.at(req.uri().path()) {
            // A route was found. Attempt to parse the parameters and run the handler. If the
            // parameters are invalid (eg. invalid percent-encoded utf8), reply with error.
            Ok(route) => match RouteParamsExt::try_from(route.params) {
                Ok(params) => Ok((route.value, Some(params))),
                Err(err) => Err(RouteError::Path(err)),
            },
            // No route was found. Use the fallback if it exists, otherwise reply with error.
            Err(MatchError::NotFound) => match self.router.default {
                Some(ref route) => Ok((route, None)),
                None => Err(RouteError::NotFound),
            },

            // There was either a trailing slash when there shouldn't be, or there wasn't a trailing
            // slash when there should be. Reply with an error that should redirect the user to the
            // correct path.
            Err(MatchError::ExtraTrailingSlash) => {
                uri = replace_path(req.uri(), req.uri().path().strip_suffix('/').unwrap());
                Err(RouteError::Expected(&uri))
            }
            Err(MatchError::MissingTrailingSlash) => {
                uri = replace_path(req.uri(), format_args!("{}/", req.uri().path()));
                Err(RouteError::Expected(&uri))
            }
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
            Err(err) => RequestFuture::Response(Some(self.formatter.clone().format_error(err))),
        }
    }
}

#[pin_project(project = RequestFutureProj)]
pub enum RequestFuture<Res, Fut> {
    Route(#[pin] Fut),
    Response(Option<Res>),
}

impl<Res, Fut> Future for RequestFuture<Res, Fut>
where
    Fut: Future<Output = Res>,
{
    type Output = Result<Res, Infallible>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().project() {
            RequestFutureProj::Route(fut) => return Poll::Ready(Ok(ready!(fut.poll(cx)))),
            RequestFutureProj::Response(res @ Some(_)) => Poll::Ready(Ok(res.take().unwrap())),
            RequestFutureProj::Response(None) => panic!("future polled after completion"),
        }
    }
}
