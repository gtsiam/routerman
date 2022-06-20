use std::{collections::HashMap, fmt::Debug};

pub struct RouteParams(pub(crate) HashMap<Box<str>, Box<str>>);

impl RouteParams {
    pub fn get(&self, param: impl AsRef<str>) -> Option<&str> {
        self.0.get(param.as_ref()).map(|v| &**v)
    }
}

impl Debug for RouteParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.0.iter()).finish()
    }
}
