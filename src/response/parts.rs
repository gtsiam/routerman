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
    ($($ty:ident),*) => {
        impl<Res, Fmt, $($ty,)* R> IntoResponse<Res, Fmt> for ($($ty,)* R)
        where
            $($ty: ResponsePart<Res, Fmt>,)*
            R: IntoResponse<Res, Fmt>,
        {
            fn into_response(self, fmt: Fmt) -> (Res, Option<Fmt>) {
                #[allow(non_snake_case)]
                let ($($ty,)* R) = self;

                let (res, fmt) = respond!(R, fmt);
                $(let (res, fmt) = respond!($ty, res, fmt);)*

                (res, Some(fmt))
            }
        }

        impl<Res, Fmt, $($ty,)*> ResponsePart<Res, Fmt> for ($($ty,)*)
        where
            $($ty: ResponsePart<Res, Fmt>,)*
        {
            fn response_part(self, res: Res, fmt: Fmt) -> (Res, Option<Fmt>) {
                #[allow(non_snake_case)]
                let ($($ty,)*) = self;

                $(let (res, fmt) = respond!($ty, res, fmt);)*

                (res, Some(fmt))
            }
        }
    };
}

macro_rules! all_the_tuples {
    ($name:ident) => {
        $name!();
        $name!(T1);
        $name!(T1, T2);
        $name!(T1, T2, T3);
        $name!(T1, T2, T3, T4);
        $name!(T1, T2, T3, T4, T5);
        $name!(T1, T2, T3, T4, T5, T6);
        $name!(T1, T2, T3, T4, T5, T6, T7);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
    };
}

all_the_tuples!(impl_response_parts);
