use nom::{Finish, IResult};
use std::fmt::{self, Debug, Display, Formatter};

/// An error that represents multiple causes.
#[derive(thiserror::Error, Debug)]
pub(crate) struct MultiError {
    pub(crate) errors: Vec<anyhow::Error>,
}

impl Display for MultiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Multiple errors occurred: ")?;

        for error in self.errors.iter() {
            writeln!(f, "{}", error)?;
        }

        Ok(())
    }
}

impl From<Vec<anyhow::Error>> for MultiError {
    fn from(errors: Vec<anyhow::Error>) -> MultiError {
        MultiError { errors }
    }
}

/// Run a fallible function across an iterable, returning _all_ errors encountered
/// instead of stopping early.
///
/// Think of this like std'd `impl FromIterator for Result`, except it keeps going.
pub(crate) trait TryWithProgress {
    type Item;

    /// Run a fallible function across an iterable, returning _all_ errors encountered
    /// instead of stopping early.
    ///
    /// Think of this like std'd `impl FromIterator for Result`, except it keeps going.
    fn try_with_progress<U, E>(
        self,
        f: impl FnMut(Self::Item) -> Result<U, E>,
    ) -> Result<Vec<U>, Vec<E>>;
}

impl<T, I> TryWithProgress for I
where
    I: IntoIterator<Item = T>,
{
    type Item = T;

    fn try_with_progress<U, E>(
        self,
        mut f: impl FnMut(Self::Item) -> Result<U, E>,
    ) -> Result<Vec<U>, Vec<E>> {
        let mut errs = Vec::new();
        let mut goods = Vec::new();
        for item in self.into_iter() {
            match (f)(item) {
                Ok(u) => {
                    goods.push(u);
                }
                Err(e) => {
                    errs.push(e);
                }
            }
        }
        if errs.is_empty() {
            Ok(goods)
        } else {
            Err(errs)
        }
    }
}

pub fn own_nom_err(err: nom::error::Error<&str>) -> nom::error::Error<String> {
    nom::error::Error::new(err.input.to_owned(), err.code)
}

pub fn discard_input<I, T>((_, t): (I, T)) -> T {
    t
}

pub trait NomFinalize<T> {
    fn finalize(self) -> Result<T, nom::error::Error<String>>;
}

impl<T> NomFinalize<T> for IResult<&str, T> {
    fn finalize(self) -> Result<T, nom::error::Error<String>> {
        self.finish().map(discard_input).map_err(own_nom_err)
    }
}
