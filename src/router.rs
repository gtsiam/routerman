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
    Body, StatusCode, Uri,
};
use matchit::MatchError;
use pin_project::pin_project;
use tower_service::Service;

use crate::{
    request::ext::{InvalidParamEncoding, RemoteAddrExt, RouteParamsExt},
    response::{DefaultFormatter, IntoResponse},
    route::{HandlerFuture, Route},
    Request, Response,
};

pub struct Router<'h, Fmt = DefaultFormatter, B = hyper::Body> {
    formatter: Fmt,
    inner: Arc<RouterImpl<'h, Fmt, B>>,
}

struct RouterImpl<'h, Fmt, B> {
    inner: matchit::Router<Route<'h, Fmt, B>>,
    default: Route<'h, Fmt, B>,
}

impl<'h, Fmt> Router<'h, Fmt>
where
    Fmt: Default + Clone + Send + Sync + 'h,
{
    pub fn builder() -> RouterBuilder<'h, Fmt> {
        RouterBuilder::with_formatter(Fmt::default())
    }
}

impl<'h, Fmt> Router<'h, Fmt>
where
    Fmt: Clone + Send + Sync + 'h,
{
    pub fn with_formatter(fmt: Fmt) -> RouterBuilder<'h, Fmt> {
        RouterBuilder::with_formatter(fmt)
    }
}

pub struct RouterBuilder<'h, Fmt = DefaultFormatter, B = hyper::Body> {
    formatter: Fmt,
    inner: matchit::Router<Route<'h, Fmt, B>>,
    default: Route<'h, Fmt, B>,
}

impl<'h, Fmt, B> RouterBuilder<'h, Fmt, B>
where
    Fmt: Clone + Send + Sync + 'h,
{
    pub fn with_formatter(formatter: Fmt) -> Self {
        Self {
            inner: matchit::Router::new(),
            default: Route::with_fmt(|_, fmt| async { StatusCode::NOT_FOUND.into_response(fmt) }),
            formatter,
        }
    }

    pub fn route(mut self, path: impl Into<String>, route: impl Into<Route<'h, Fmt, B>>) -> Self {
        self.inner.insert(path, route.into()).expect("insert route");
        self
    }

    pub fn build(self) -> Router<'h, Fmt, B> {
        Router {
            inner: Arc::new(RouterImpl {
                inner: self.inner,
                default: self.default,
            }),
            formatter: self.formatter,
        }
    }
}

impl<'h, Fmt, B> Service<&AddrStream> for Router<'h, Fmt, B>
where
    Fmt: Clone,
{
    type Response = RequestService<'h, Fmt, B>;
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

pub struct RequestService<'h, Fmt, B> {
    formatter: Fmt,
    remote_addr: SocketAddr,
    router: Arc<RouterImpl<'h, Fmt, B>>,
}

impl<'h, Fmt, B> Service<Request<B>> for RequestService<'h, Fmt, B>
where
    Fmt: Clone + Send + Sync + 'h,
{
    type Response = Response;
    type Error = Infallible;
    type Future = RequestFuture<'h>;

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

        enum MatchResult<'a, 'h, Fmt, B> {
            Route(&'a Route<'h, Fmt, B>, Option<RouteParamsExt>),
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
                    .into_response(self.formatter.clone()),
            )),
            MatchResult::InvalidParamEncoding(_err) => RequestFuture::Response(Some(
                StatusCode::BAD_REQUEST.into_response(self.formatter.clone()),
            )),
        }
    }
}

#[pin_project(project = RequestFutureProj)]
pub enum RequestFuture<'h> {
    Route(#[pin] HandlerFuture<'h>),
    Response(Option<Response>),
}

impl<'h> Future for RequestFuture<'h> {
    type Output = Result<Response, Infallible>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().project() {
            RequestFutureProj::Route(fut) => return Poll::Ready(Ok(ready!(fut.poll(cx)))),
            RequestFutureProj::Response(res @ Some(_)) => Poll::Ready(Ok(res.take().unwrap())),
            RequestFutureProj::Response(None) => panic!("future polled after completion"),
        }
    }
}
