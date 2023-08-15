use std::iter::Peekable;

use strum::IntoEnumIterator;

use crate::error::Result;

/// A safe wrapper for an iterator that cycles endlessly.
pub struct EnumCycle<T: IntoEnumIterator> {
    inner: Peekable<Box<dyn Iterator<Item = T>>>,
}

impl<T: IntoEnumIterator + Clone + 'static> EnumCycle<T> {
    /// Creates a new `EnumCycle` for the given type.
    /// Returns an error if the enum doesn't have at least one variant.
    pub fn new() -> Result<EnumCycle<T>> {
        let all = T::iter().collect::<Vec<T>>();
        if all.is_empty() {
            bail!("enum to cycle must contain at least one variant!");
        }

        Ok(EnumCycle {
            inner: (Box::new(all.into_iter().cycle()) as Box<dyn Iterator<Item = T>>).peekable(),
        })
    }

    pub fn current(&mut self) -> &T {
        // SAFETY: `self.inner` is a cycling iterator that has at least one variant
        self.inner.peek().unwrap()
    }

    pub fn next(&mut self) -> T {
        // SAFETY: `self.inner` is a cycling iterator that has at least one variant
        self.inner.next().unwrap()
    }
}

impl<T: IntoEnumIterator + PartialEq + Clone + 'static> EnumCycle<T> {
    /// Creates a new `EnumCycle` for the given type, starting at the given variant.
    /// Returns an error if the enum doesn't have at least one variant.
    pub fn new_at(start: T) -> Result<EnumCycle<T>> {
        let mut me = Self::new()?;
        while *me.current() != start {
            me.next();
        }

        Ok(me)
    }
}

impl<T: IntoEnumIterator + PartialEq + Default + Clone + 'static> EnumCycle<T> {
    /// Creates a new `EnumCycle` for the given type, starting at the default variant.
    /// Returns an error if the enum doesn't have at least one variant.
    pub fn new_at_default() -> Result<EnumCycle<T>> {
        let start = T::default();
        let mut me = Self::new()?;
        while *me.current() != start {
            me.next();
        }

        Ok(me)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_iterator() {
        #[derive(Debug, Clone, strum::EnumIter)]
        enum Empty {}

        // should not allow an empty iterator
        match EnumCycle::<Empty>::new() {
            Ok(_) => panic!("should not be Ok"),
            Err(e) => assert_eq!(
                e.to_string(),
                "enum to cycle must contain at least one variant!"
            ),
        }
    }
}
