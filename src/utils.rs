use std::iter::FromIterator;

/// Run a fallible function across an iterable, returning _all_ errors encountered
/// instead of stopping early.
///
/// Think of this like std'd `impl FromIterator for Result`, except it keeps going.
pub fn try_with_progress<T, U, E, I: FromIterator<U>>(
    values: impl IntoIterator<Item = T>,
    mut f: impl FnMut(T) -> Result<U, E>,
) -> Result<I, Vec<E>> {
    let mut errs = Vec::new();
    let mut goods = Vec::new();
    for item in values.into_iter() {
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
        Ok(goods.into_iter().collect())
    } else {
        Err(errs)
    }
}
