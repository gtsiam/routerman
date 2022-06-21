use super::{Reply, ReplyPart, Response};

macro_rules! impl_response_parts {
    () => { };
    ($($ty:ident),*) => {
        impl<Fmt, $($ty,)*> Reply<Fmt> for ($($ty,)*)
        where
            $($ty: ReplyPart<Fmt>,)*
        {
            fn reply(self, fmt: Fmt) -> Response {
                #[allow(non_snake_case)]
                let ($($ty,)*) = self;

                let res = Response::default();
                $(
                    #[allow(unused)]
                    let (res, fmt) = match $ty.reply_part(res, fmt) {
                        (res, Some(fmt)) => (res, fmt),
                        (res, None) => return res,
                    };
                )*

                res
            }
        }

        impl<Fmt, $($ty),*> ReplyPart<Fmt> for ($($ty,)*)
        where
            $($ty: ReplyPart<Fmt>,)*
        {
            fn reply_part(self, res: Response, fmt: Fmt) -> (Response, Option<Fmt>) {
                #[allow(non_snake_case)]
                let ($($ty,)*) = self;

                $(let (res, fmt) = match $ty.reply_part(res, fmt) {
                    (res, Some(fmt)) => (res, fmt),
                    r => return r,
                };)*

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
