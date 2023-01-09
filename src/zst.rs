use crate::sync::atomic::{AtomicUsize, Ordering};
use crate::sync::prelude::*;
use crate::{PopError, PushError};

use core::marker::PhantomData;
use core::mem;
use core::num::NonZeroUsize;

/// A concurrent queue consisting of ZSTs.
///
/// This allows us to simplify ZST operation down to fewer atomic operations.
pub(super) struct Zst<T> {
    /// Capture the type of the ZST.
    _marker: PhantomData<T>,

    /// The current state of the queue.
    ///
    /// The lowest bit indicates whether the queue is closed. The rest of the bits
    /// indicate the number of items in the queue.
    state: AtomicUsize,

    /// The capacity of the queue.
    capacity: Option<NonZeroUsize>,
}

/// The state of the ZST queue.
///
/// This can be serialized to and from a `usize`.
#[derive(Copy, Clone)]
struct State {
    /// The number of items in the queue.
    count: usize,

    /// Whether the queue is closed.
    closed: bool,
}

const CLOSED: usize = 1 << 0;
const REFCOUNT_SHIFT: usize = 1;

impl From<usize> for State {
    fn from(state: usize) -> Self {
        Self {
            count: state >> REFCOUNT_SHIFT,
            closed: state & CLOSED != 0,
        }
    }
}

impl From<State> for usize {
    fn from(state: State) -> Self {
        state.count << REFCOUNT_SHIFT | if state.closed { CLOSED } else { 0 }
    }
}

impl<T> Zst<T> {
    /// Create a new queue from a capacity.
    pub(super) fn new(capacity: Option<usize>) -> Self {
        assert_eq!(mem::size_of::<T>(), 0);

        Self {
            _marker: PhantomData,
            state: AtomicUsize::new(0),
            capacity: capacity.map(|capacity| {
                NonZeroUsize::new(capacity).unwrap_or_else(|| {
                    panic!("capacity must be positive");
                })
            }),
        }
    }

    /// Attempts to push an item into the queue.
    pub(super) fn push(&self, value: T) -> Result<(), PushError<T>> {
        // Load the current state.
        let mut state = State::from(self.state.load(Ordering::Acquire));

        loop {
            // If we are closed, error out.
            if state.closed {
                return Err(PushError::Closed(value));
            }

            // If we have a capacity, check if we are full.
            if let Some(capacity) = self.capacity {
                if state.count >= capacity.get() {
                    return Err(PushError::Full(value));
                }
            }

            // Otherwise, increase the count.
            let new_state = State {
                count: state.count + 1,
                ..state
            };

            // Avoid a potential overflow.
            if new_state.count >= core::usize::MAX >> REFCOUNT_SHIFT {
                return Err(PushError::Full(value));
            }

            // Serialize the new state.
            if self
                .state
                .compare_exchange(
                    state.into(),
                    new_state.into(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_err()
            {
                // If the CAS failed, reload the state and try again.
                state = self.state.load(Ordering::Acquire).into();
                continue;
            }

            // We successfully pushed an item, make sure `value`'s drop handle doesn't run.
            mem::forget(value);
            return Ok(());
        }
    }

    /// Attempts to pop an item from the queue.
    pub(super) fn pop(&self) -> Result<T, PopError> {
        // Load the current state.
        let mut state = State::from(self.state.load(Ordering::Acquire));

        loop {
            // If we are empty, error out.
            if state.count == 0 {
                if state.closed {
                    return Err(PopError::Closed);
                } else {
                    return Err(PopError::Empty);
                }
            }

            // Otherwise, decrease the count.
            let new_state = State {
                count: state.count - 1,
                ..state
            };

            // Serialize the new state.
            if self
                .state
                .compare_exchange(
                    state.into(),
                    new_state.into(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_err()
            {
                // If the CAS failed, reload the state and try again.
                state = self.state.load(Ordering::Acquire).into();
                continue;
            }

            // We successfully popped an item.
            return Ok(instance());
        }
    }

    /// Returns `true` is the queue is empty.
    pub(super) fn is_empty(&self) -> bool {
        State::from(self.state.load(Ordering::Acquire)).count == 0
    }

    /// Returns `true` if the queue is full.
    pub(super) fn is_full(&self) -> bool {
        if let Some(capacity) = self.capacity {
            State::from(self.state.load(Ordering::Acquire)).count >= capacity.get()
        } else {
            false
        }
    }

    /// Returns the number of items in the queue.
    pub(super) fn len(&self) -> usize {
        State::from(self.state.load(Ordering::Acquire)).count
    }

    /// Returns the capacity of the queue.
    pub(super) fn capacity(&self) -> Option<usize> {
        self.capacity.map(|c| c.get())
    }

    /// Closes the queue, and returns `true` if the queue was previously open.
    pub(super) fn close(&self) -> bool {
        let state = State::from(self.state.fetch_or(CLOSED, Ordering::SeqCst));
        !state.closed
    }

    /// Returns `true` if the queue is closed.
    pub(super) fn is_closed(&self) -> bool {
        State::from(self.state.load(Ordering::Acquire)).closed
    }
}

impl<T> Drop for Zst<T> {
    fn drop(&mut self) {
        // Make sure the ZST's drop handlers are run.
        self.state.with_mut(|&mut state| {
            for _ in 0..State::from(state).count {
                drop(instance::<T>());
            }
        })
    }
}

/// Instantiate a new instance of a ZST type.
#[inline(always)]
fn instance<T>() -> T {
    assert_eq!(mem::size_of::<T>(), 0);

    // SAFETY: We know that the size of T is zero, so a valid instance can be zeroed.
    unsafe { mem::zeroed() }
}
