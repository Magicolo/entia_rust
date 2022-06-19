use crate::{
    error::{Error, Result},
    recurse,
};

pub trait IntoOutput {
    fn output(self) -> Result;
}

impl IntoOutput for Error {
    #[inline]
    fn output(self) -> Result {
        Err(self)
    }
}

impl<T: IntoOutput> IntoOutput for Option<T> {
    #[inline]
    fn output(self) -> Result {
        self.map_or(Ok(()), IntoOutput::output)
    }
}

impl<T: IntoOutput> IntoOutput for Result<T> {
    #[inline]
    fn output(self) -> Result {
        self.and_then(IntoOutput::output)
    }
}

macro_rules! output {
    ($($p:ident, $t:ident),*) => {
        impl<'a, $($t: IntoOutput,)*> IntoOutput for ($($t,)*) {
            #[inline]
            fn output(self) -> Result {
                let ($($p,)*) = self;
                $($p.output()?;)*
                Ok(())
            }
        }
    };
}

recurse!(output);
