use std::cell::UnsafeCell;
use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::rc::Rc;

/// A special cell which can be used to share multiple references to the same
/// underlying type. The references allow mutation of the inner type. This type
/// isn't `Send` at all - in fact, its safety is closely linked with that of a
/// single-threaded program - though multiple mutable references may exist to
/// it, only one may actually mutate at a time.
///
/// This isn't a safe type by any means: two `RcCell`'s can exist which both
/// use `rc_cell.get_mut()` at the same time - that means two mutable references!
///
/// For our use case though (single-threaded context) it's convenient and safe.
pub struct RcCell<T> {
    inner: Rc<UnsafeCell<T>>,
}

impl<T> RcCell<T> {
    pub fn new(inner: T) -> RcCell<T> {
        RcCell {
            inner: Rc::new(UnsafeCell::new(inner)),
        }
    }

    /// Get an immutable reference to the inner type.
    fn get(&self) -> &T {
        unsafe { &*self.inner.get() }
    }

    /// Get a mutable reference to the inner type.
    fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}

impl<T> Deref for RcCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> DerefMut for RcCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T> Debug for RcCell<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Wrapper")
            .field(unsafe { &*self.inner.get() })
            .finish()
    }
}

impl<T> Display for RcCell<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", unsafe { &*self.inner.get() }))
    }
}

impl<T> Clone for RcCell<T> {
    fn clone(&self) -> Self {
        RcCell {
            inner: self.inner.clone(),
        }
    }
}

impl<T> PartialEq for RcCell<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl<T> Eq for RcCell<T> where T: Eq {}

impl<T, I> Index<I> for RcCell<T>
where
    T: Index<I>,
{
    type Output = T::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.get()[index]
    }
}

impl<T, I> IndexMut<I> for RcCell<T>
where
    T: IndexMut<I>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.get_mut()[index]
    }
}

#[cfg(test)]
mod cell_tests {
    use futures::stream::FuturesUnordered;
    use futures::StreamExt;
    use tokio::join;

    use super::*;
    use crate::util::local_block_on;

    #[test]
    fn across_await_boundaries() {
        local_block_on(async {
            let data = RcCell::new(0);

            let (tx1, rx1) = tokio::sync::oneshot::channel();
            let (tx2, rx2) = tokio::sync::oneshot::channel();

            let fut1 = {
                let mut data = data.clone();
                async move {
                    let mutable_ref = data.get_mut();
                    tx1.send(()).unwrap();
                    rx2.await.unwrap();
                    *mutable_ref += 1;
                }
            };

            let fut2 = {
                let mut data = data.clone();
                async move {
                    let mutable_ref = data.get_mut();
                    tx2.send(()).unwrap();
                    rx1.await.unwrap();
                    *mutable_ref += 1;
                }
            };

            assert_eq!(*data, 0);
            join!(fut1, fut2);
            assert_eq!(*data, 2);
        })
        .unwrap();
    }

    #[test]
    fn multiple_mutable_futures() {
        local_block_on(async {
            let data = RcCell::new(0);

            let futures = FuturesUnordered::new();
            for _ in 0..100 {
                futures.push({
                    let mut data = data.clone();
                    async move { *data.get_mut() += 1 }
                });
            }

            futures.collect::<Vec<_>>().await;
            assert_eq!(*data, 100);
        })
        .unwrap();
    }

    #[test]
    fn deref() {
        let data = RcCell::new(vec![1, 2, 3, 4]);
        let interface = |slice: &[i32]| assert_eq!(slice, &[1, 2, 3, 4]);
        interface(&data);
    }

    #[test]
    fn clone_and_move() {
        let mut data = RcCell::new(String::from("hello"));
        let f = {
            let data = data.clone();
            move || data
        };

        // assert initial state
        assert_eq!(data.get(), "hello");
        // mutate with first reference
        data.get_mut().push_str(", world!");
        // get second reference from closure, and check mutation
        assert_eq!(f().get(), "hello, world!");
    }

    #[test]
    fn index() {
        let mut bar = RcCell::new(vec![String::new()]);
        // assert index
        assert_eq!(&bar[0], "");
        // do index mut
        bar[0] = String::from("hello");
        // assert mutation
        assert_eq!(&bar[0], "hello");
    }
}
