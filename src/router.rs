use core::fmt::Display;
use std::{
    convert::Infallible,
    future::{Future, Ready},
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_util::ready;
use hyper::{
    header::{self},
    server::conn::AddrStream,
    Body, Request, Response, StatusCode, Uri,
};
use matchit::MatchError;
use pin_project::pin_project;
use tower_service::Service;

use crate::{
    request::ext::{InvalidParamEncoding, RemoteAddrExt, RouteParamsExt},
    response::{DefaultFormatter, Formatter, IntoResponse},
    route::{HandlerFuture, Route},
};

pub struct Router<'h, Req, Res, Fmt = DefaultFormatter> {
    formatter: Fmt,
    inner: Arc<RouterImpl<'h, Req, Res, Fmt>>,
}

struct RouterImpl<'h, Req, Res, Fmt> {
    inner: matchit::Router<Route<'h, Req, Res, Fmt>>,
    default: Route<'h, Req, Res, Fmt>,
}

impl<'h, Req, Res, Fmt> Router<'h, Req, Res, Fmt>
where
    Req: Send + 'static,
    Fmt: Default + Clone + Send + Sync + 'h,
{
    pub fn builder() -> RouterBuilder<'h, Req, Res, Fmt> {
        RouterBuilder::with_formatter(Fmt::default())
    }
}

impl<'h, Req, Res, Fmt> Router<'h, Req, Res, Fmt>
where
    Req: Send + 'static,
    Fmt: Clone + Send + Sync + 'h,
{
    pub fn with_formatter(fmt: Fmt) -> RouterBuilder<'h, Req, Res, Fmt> {
        RouterBuilder::with_formatter(fmt)
    }
}

pub struct RouterBuilder<'h, Req, Res, Fmt = DefaultFormatter> {
    formatter: Fmt,
    inner: matchit::Router<Route<'h, Req, Res, Fmt>>,
    default: Route<'h, Req, Res, Fmt>,
}

impl<'h, Req, Res, Fmt> RouterBuilder<'h, Req, Res, Fmt>
where
    Req: Send + 'static,
    Fmt: Clone + Send + Sync + 'h,
{
    pub fn with_formatter(formatter: Fmt) -> Self {
        async fn default_fallback<Req>(_req: Req) -> Infallible {
            panic!("No default route")
        }

        Self {
            inner: matchit::Router::new(),
            default: Route::new(default_fallback),
            formatter,
        }
    }

    pub fn route(
        mut self,
        path: impl Into<String>,
        route: impl Into<Route<'h, Req, Res, Fmt>>,
    ) -> Self {
        self.inner.insert(path, route.into()).expect("insert route");
        self
    }

    pub fn build(self) -> Router<'h, Req, Res, Fmt> {
        Router {
            inner: Arc::new(RouterImpl {
                inner: self.inner,
                default: self.default,
            }),
            formatter: self.formatter,
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

impl<'h, Fmt, B> Service<Request<B>> for RequestService<'h, Request<B>, Response<Body>, Fmt>
where
    Fmt: Formatter<Response<Body>, hyper::http::Error> + Clone + Send + Sync + 'h,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = RequestFuture<'h, Self::Response>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
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

        req.extensions_mut()
            .insert(RemoteAddrExt::from(self.remote_addr));

        enum MatchResult<'a, 'h, Req, Res, Fmt> {
            Route(&'a Route<'h, Req, Res, Fmt>, Option<RouteParamsExt>),
            Redirect(Uri),
            InvalidParamEncoding(InvalidParamEncoding),
        }

        let res = match self.router.inner.at(req.uri().path()) {
            Ok(route) => match route.params.try_into() {
                Ok(params) => MatchResult::Route(route.value, Some(params)),
                Err(err) => MatchResult::InvalidParamEncoding(err),
            },
            Err(MatchError::NotFound) => MatchResult::Route(&self.router.default, None),

            Err(MatchError::ExtraTrailingSlash) => MatchResult::Redirect(replace_path(
                req.uri(),
                req.uri().path().strip_suffix('/').unwrap(),
            )),
            Err(MatchError::MissingTrailingSlash) => MatchResult::Redirect(replace_path(
                req.uri(),
                format_args!("{}/", req.uri().path()),
            )),
        };

        match res {
            MatchResult::Route(route, params) => {
                if let Some(params) = params {
                    req.extensions_mut().insert(params);
                }

                RequestFuture::Route((route.handler())(req, self.formatter.clone()))
            }
            MatchResult::Redirect(uri) => RequestFuture::Response(Some(
                Response::builder()
                    .status(StatusCode::PERMANENT_REDIRECT)
                    .header(header::LOCATION, uri.to_string())
                    .body(Body::empty())
                    .into_response(self.formatter.clone())
                    .0,
            )),
            MatchResult::InvalidParamEncoding(_err) => RequestFuture::Response(Some(
                StatusCode::BAD_REQUEST
                    .into_response(self.formatter.clone())
                    .0,
            )),
        }
    }
}

#[pin_project(project = RequestFutureProj)]
pub enum RequestFuture<'h, Res> {
    Route(#[pin] HandlerFuture<'h, Res>),
    Response(Option<Res>),
}

impl<'h, Res> Future for RequestFuture<'h, Res> {
    type Output = Result<Res, Infallible>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().project() {
            RequestFutureProj::Route(fut) => return Poll::Ready(Ok(ready!(fut.poll(cx)))),
            RequestFutureProj::Response(res @ Some(_)) => Poll::Ready(Ok(res.take().unwrap())),
            RequestFutureProj::Response(None) => panic!("future polled after completion"),
        }
    }
}
