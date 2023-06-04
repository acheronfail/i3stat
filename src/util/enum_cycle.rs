use std::iter::Peekable;

use strum::IntoEnumIterator;

pub struct EnumCycle<T: IntoEnumIterator> {
    inner: Peekable<Box<dyn Iterator<Item = T>>>,
}

impl<T: IntoEnumIterator + Clone + 'static> EnumCycle<T> {
    pub fn new() -> EnumCycle<T> {
        let all = T::iter().collect::<Vec<T>>();
        EnumCycle {
            inner: (Box::new(all.into_iter().cycle()) as Box<dyn Iterator<Item = T>>).peekable(),
        }
    }

    pub fn current(&mut self) -> &T {
        self.inner.peek().unwrap()
    }

    pub fn next(&mut self) -> T {
        self.inner.next().unwrap()
    }
}

impl<T: IntoEnumIterator + PartialEq + Clone + 'static> EnumCycle<T> {
    pub fn new_at(start: T) -> EnumCycle<T> {
        let mut me = Self::new();
        while *me.current() != start {
            me.next();
        }

        me
    }
}

impl<T: IntoEnumIterator + PartialEq + Default + Clone + 'static> EnumCycle<T> {
    pub fn new_at_default() -> EnumCycle<T> {
        let start = T::default();
        let mut me = Self::new();
        while *me.current() != start {
            me.next();
        }

        me
    }
}
