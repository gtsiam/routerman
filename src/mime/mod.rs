use hyper::header::HeaderValue;

pub struct Mime<'s> {
    source: &'s str,
}

impl Mime<'_> {
    pub const fn as_str(&self) -> &str {
        self.source
    }

    pub const fn header(&'static self) -> HeaderValue {
        HeaderValue::from_static(self.as_str())
    }
}

pub const TEXT_PLAIN: Mime<'static> = Mime {
    source: "text/plain",
};
pub const APPLICATION_JSON: Mime<'static> = Mime {
    source: "application/json",
};
