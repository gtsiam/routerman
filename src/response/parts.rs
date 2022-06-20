use super::{IntoResponse, ResponsePart};
use crate::respond;

macro_rules! impl_response_parts {
    () => {
        impl<Res, Fmt, R> IntoResponse<Res, Fmt> for (R,)
        where
            R: IntoResponse<Res, Fmt>,
        {
            fn into_response(self, fmt: Fmt) -> (Res, Option<Fmt>) {
                self.0.into_response(fmt)
            }
        }
    };
    ($first:ident $(, $rest:ident)*) => {
        impl_response_parts!($($rest),*);

        impl<Res, Fmt, $first, $($rest,)* R> IntoResponse<Res, Fmt> for ($first, $($rest,)* R)
        where
            $first: ResponsePart<Res, Fmt>,
            $($rest: ResponsePart<Res, Fmt>,)*
            R: IntoResponse<Res, Fmt>,
        {
            fn into_response(self, fmt: Fmt) -> (Res, Option<Fmt>) {
                #[allow(non_snake_case)]
                let ($first, $($rest,)* R) = self;

                let (res, fmt) = respond!(R, fmt);
                let (res, fmt) = respond!($first, res, fmt);
                $(let (res, fmt) = respond!($rest, res, fmt);)*

                (res, Some(fmt))
            }
        }

        impl<Res, Fmt, $first, $($rest),*> ResponsePart<Res, Fmt> for ($first, $($rest),*)
        where
            $first: ResponsePart<Res, Fmt>,
            $($rest: ResponsePart<Res, Fmt>,)*
        {
            fn response_part(self, res: Res, fmt: Fmt) -> (Res, Option<Fmt>) {
                #[allow(non_snake_case)]
                let ($first, $($rest),*) = self;

                let (res, fmt) = respond!($first, res, fmt);
                $(let (res, fmt) = respond!($rest, res, fmt);)*

                (res, Some(fmt))
            }
        }
    };
}

impl_response_parts!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
