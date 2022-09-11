//! Synchronization facade to choose between `core` primitives and `loom` primitives.

#[cfg(all(feature = "portable-atomic", not(loom)))]
mod sync_impl {
    pub(crate) use core::cell;
    pub(crate) use portable_atomic as atomic;
}

#[cfg(all(not(feature = "portable-atomic"), not(loom)))]
mod sync_impl {
    pub(crate) use core::cell;
    pub(crate) use core::sync::atomic;
}

#[cfg(loom)]
mod sync_impl {
    pub(crate) use loom::cell;

    pub(crate) mod atomic {
        pub(crate) use core::sync::atomic::compiler_fence;
        pub(crate) use loom::sync::atomic::*;
    }
}

pub(crate) use sync_impl::*;

#[cfg(loom)]
pub(crate) mod prelude {}

#[cfg(not(loom))]
pub(crate) mod prelude {
    use super::{atomic, cell};

    /// Emulate `loom::UnsafeCell`'s API.
    pub(crate) trait UnsafeCellExt {
        type Value;

        fn with<R, F>(&self, f: F) -> R
        where
            F: FnOnce(*const Self::Value) -> R;

        fn with_mut<R, F>(&self, f: F) -> R
        where
            F: FnOnce(*mut Self::Value) -> R;
    }

    impl<T> UnsafeCellExt for cell::UnsafeCell<T> {
        type Value = T;

        fn with<R, F>(&self, f: F) -> R
        where
            F: FnOnce(*const Self::Value) -> R,
        {
            f(self.get())
        }

        fn with_mut<R, F>(&self, f: F) -> R
        where
            F: FnOnce(*mut Self::Value) -> R,
        {
            f(self.get())
        }
    }

    /// Emulate `loom::Atomic*`'s API.
    pub(crate) trait AtomicExt {
        type Value;

        fn with_mut<R, F>(&mut self, f: F) -> R
        where
            F: FnOnce(&mut Self::Value) -> R;
    }

    impl AtomicExt for atomic::AtomicUsize {
        type Value = usize;

        fn with_mut<R, F>(&mut self, f: F) -> R
        where
            F: FnOnce(&mut Self::Value) -> R,
        {
            f(self.get_mut())
        }
    }

    impl<T> AtomicExt for atomic::AtomicPtr<T> {
        type Value = *mut T;

        fn with_mut<R, F>(&mut self, f: F) -> R
        where
            F: FnOnce(&mut Self::Value) -> R,
        {
            f(self.get_mut())
        }
    }
}
