use super::params::RouteParams;
use std::{
    net::SocketAddr,
    ops::{Deref, DerefMut},
};
use thiserror::Error;

pub struct RemoteAddrExt(pub SocketAddr);

impl Deref for RemoteAddrExt {
    type Target = SocketAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RemoteAddrExt {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<RemoteAddrExt> for SocketAddr {
    fn from(ext: RemoteAddrExt) -> Self {
        ext.0
    }
}

pub struct RouteParamsExt(RouteParams);

impl Deref for RouteParamsExt {
    type Target = RouteParams;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RouteParamsExt {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<RouteParamsExt> for RouteParams {
    fn from(ext: RouteParamsExt) -> Self {
        ext.0
    }
}

#[derive(Debug, Error)]
#[error("invalid param encoding for key `{0}`")]
pub struct InvalidParamEncoding(Box<str>);

impl<'k, 'v> TryFrom<matchit::Params<'k, 'v>> for RouteParamsExt {
    type Error = InvalidParamEncoding;

    fn try_from(params: matchit::Params<'k, 'v>) -> Result<Self, Self::Error> {
        Ok(Self(RouteParams(
            params
                .iter()
                .map(|(k, v)| {
                    percent_encoding::percent_decode(v.as_bytes())
                        .decode_utf8()
                        .map(|decoded| (Box::from(k), Box::from(decoded)))
                        .map_err(|_| InvalidParamEncoding(Box::from(k)))
                })
                .collect::<Result<_, _>>()?,
        )))
    }
}
