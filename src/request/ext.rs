use std::{net::SocketAddr, ops::Deref};

use thiserror::Error;

use super::params::RouteParams;

pub struct RemoteAddrExt(SocketAddr);

impl Deref for RemoteAddrExt {
    type Target = SocketAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<SocketAddr> for RemoteAddrExt {
    fn from(addr: SocketAddr) -> Self {
        Self(addr)
    }
}

pub struct RouteParamsExt(RouteParams);

impl Deref for RouteParamsExt {
    type Target = RouteParams;

    fn deref(&self) -> &Self::Target {
        &self.0
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
