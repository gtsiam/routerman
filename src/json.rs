//! Utilities for json serialization and deserialization

use crate::mime;
use crate::request::extract::ExtractFrom;
use crate::response::{DefaultFormatter, Formatter, IntoResponse};
use futures_util::Future;
use hyper::body::HttpBody;
use hyper::{body::Bytes, header, Body, Response};
use hyper::{Request, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::fmt::{Debug, Display};
use std::future::{ready, Ready};
use std::pin::Pin;

/// Helper for managing json responses and requests
///
/// Wrapper type for serializable and/or deserializable objects that can be extracted from a request
/// or converted into a response.
///
/// To extract information from a request, you have to use the [`ExtractFrom::extract_from`]
/// function, which will take in a hyper request and return the deserialized Json object.
///
/// Returning a json object is also possible, in which case it will automatically be converted into
/// a response.
/// ```
/// # use hyper::{Request, Body, body::Bytes};
/// # use routerman::{
/// #   route::Route, json::Json,
/// #   request::extract::ExtractFrom, response::{IntoResponse, DefaultFormatter},
/// # };
/// # use std::collections::HashMap;
/// Route::<_, _, DefaultFormatter>::new(|req: Request<Body>| async move {
///     // Parse json from user input
///     let data: HashMap<String, String> = Json::extract_from(req).await.unwrap().0;
///
///     // Echo back the user's request
///     Json(data)
/// });
/// ```
///
/// Performing zero-copy deserialization is possible, but you will have to manually extract the
/// Bytes object the deserialized json will reference and maintain it for the duration of the
/// request.
/// ```
/// # use hyper::{Request, Body, body::Bytes};
/// # use routerman::{
/// #   route::Route, json::Json,
/// #   request::extract::ExtractFrom, response::{IntoResponse, DefaultFormatter},
/// # };
/// # use std::collections::HashMap;
/// Route::<_, _, DefaultFormatter>::new(|req: Request<Body>| async move {
///     let bytes = Bytes::extract_from(req).await.unwrap();
///     let data: HashMap<&str, &str> = Json::extract_from(&bytes).await.unwrap().0;
///
///     // Json(data) // Error: See below
/// });
/// ```
///
/// However, because the json object is now referencing a local variable it cannot be returned to
/// the user. This can be circumvented by manually calling [`IntoResponse::into_response`] and
/// then returning the response object:
/// ```
/// # use hyper::{Request, Body, body::Bytes};
/// # use routerman::{
/// #   route::Route, json::Json,
/// #   request::extract::ExtractFrom, response::{IntoResponse, DefaultFormatter},
/// # };
/// # use std::collections::HashMap;
/// Route::<_, _, DefaultFormatter>::with_fmt(|req: Request<Body>, fmt| async move {
///     let bytes = Bytes::extract_from(req).await.unwrap();
///     let data: HashMap<&str, &str> = Json::extract_from(&bytes).await.unwrap().0;
///
///     Json(data).into_response(fmt).0
/// });
/// ```
#[derive(Debug, Clone)]
pub struct Json<T>(
    /// The stored json object
    pub T,
);

pub use serde_json::Error as JsonError;

/// Json processing error
pub enum Error<B: HttpBody> {
    Body(B::Error),
    Json(JsonError),
}

impl<B: HttpBody> Display for Error<B>
where
    B::Error: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Body(err) => write!(f, "body error: {}", err),
            Error::Json(err) => write!(f, "json error: {}", err),
        }
    }
}

impl<B: HttpBody> Debug for Error<B>
where
    B::Error: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Body(arg0) => f.debug_tuple("Body").field(arg0).finish(),
            Self::Json(arg0) => f.debug_tuple("Json").field(arg0).finish(),
        }
    }
}

impl<B: HttpBody> StdError for Error<B>
where
    B::Error: StdError + 'static,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Body(err) => Some(err),
            Error::Json(err) => Some(err),
        }
    }
}

impl Formatter<Response<Body>, serde_json::Error> for DefaultFormatter {
    fn format_error(self, err: serde_json::Error) -> Response<Body> {
        (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            .into_response(self)
            .0
    }
}

impl<T, Fmt> IntoResponse<Response<Body>, Fmt> for Json<T>
where
    T: Serialize,
    Fmt: Formatter<Response<Body>, serde_json::Error>
        + Formatter<Response<Body>, hyper::http::Error>,
{
    fn into_response(self, fmt: Fmt) -> (Response<Body>, Option<Fmt>) {
        match serde_json::to_vec(&self.0) {
            Ok(content) => (
                [(header::CONTENT_TYPE, mime::APPLICATION_JSON.header())],
                Response::new(Body::from(content)),
            )
                .into_response(fmt),
            Err(err) => (fmt.format_error(err), None),
        }
    }
}

impl<'a, T> ExtractFrom<&'a Bytes> for Json<T>
where
    T: Deserialize<'a>,
{
    type Error = JsonError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn extract_from(bytes: &'a Bytes) -> Self::Future {
        ready(serde_json::from_slice(bytes.as_ref()).map(Json))
    }
}

impl<T, B> ExtractFrom<Request<B>> for Json<T>
where
    T: DeserializeOwned,
    B: HttpBody + Send + 'static,
    B::Data: Send,
{
    type Error = Error<B>;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>> + Send + 'static>>;

    fn extract_from(req: Request<B>) -> Self::Future {
        Box::pin(async move {
            let bytes = Bytes::extract_from(req).await.map_err(Error::Body)?;
            serde_json::from_slice(bytes.as_ref())
                .map_err(Error::Json)
                .map(Json)
        })
    }
}
