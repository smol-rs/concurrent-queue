#![allow(clippy::bool_assert_comparison)]

use concurrent_queue::{ConcurrentQueue, PopError, PushError};
use easy_parallel::Parallel;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn smoke() {
    let q = ConcurrentQueue::bounded(2);

    q.push(()).unwrap();
    assert_eq!(q.pop(), Ok(()));

    q.push(()).unwrap();
    assert_eq!(q.pop(), Ok(()));
    assert!(q.pop().is_err());
}

#[test]
fn smoke_unbounded() {
    let q = ConcurrentQueue::unbounded();

    q.push(()).unwrap();
    assert_eq!(q.pop(), Ok(()));

    q.push(()).unwrap();
    assert_eq!(q.pop(), Ok(()));
    assert!(q.pop().is_err());
}

#[test]
fn capacity() {
    for i in 1..10 {
        let q = ConcurrentQueue::<()>::bounded(i);
        assert_eq!(q.capacity(), Some(i));
    }
}

#[test]
#[should_panic(expected = "capacity must be positive")]
fn zero_capacity() {
    let _ = ConcurrentQueue::<()>::bounded(0);
}

#[test]
fn len_empty_full() {
    let q = ConcurrentQueue::<()>::bounded(2);

    assert_eq!(q.len(), 0);
    assert_eq!(q.is_empty(), true);
    assert_eq!(q.is_full(), false);

    q.push(()).unwrap();

    assert_eq!(q.len(), 1);
    assert_eq!(q.is_empty(), false);
    assert_eq!(q.is_full(), false);

    q.push(()).unwrap();

    assert_eq!(q.len(), 2);
    assert_eq!(q.is_empty(), false);
    assert_eq!(q.is_full(), true);

    q.pop().unwrap();

    assert_eq!(q.len(), 1);
    assert_eq!(q.is_empty(), false);
    assert_eq!(q.is_full(), false);
}

#[test]
fn len_empty_full_unbounded() {
    let q = ConcurrentQueue::<()>::unbounded();

    assert_eq!(q.len(), 0);
    assert_eq!(q.is_empty(), true);
    assert_eq!(q.is_full(), false);

    q.push(()).unwrap();

    assert_eq!(q.len(), 1);
    assert_eq!(q.is_empty(), false);
    assert_eq!(q.is_full(), false);

    q.push(()).unwrap();

    assert_eq!(q.len(), 2);
    assert_eq!(q.is_empty(), false);
    assert_eq!(q.is_full(), false);

    q.pop().unwrap();

    assert_eq!(q.len(), 1);
    assert_eq!(q.is_empty(), false);
    assert_eq!(q.is_full(), false);
}

#[test]
fn len() {
    const COUNT: usize = if cfg!(miri) { 50 } else { 25_000 };
    const CAP: usize = if cfg!(miri) { 50 } else { 1000 };

    let q = ConcurrentQueue::<()>::bounded(CAP);
    assert_eq!(q.len(), 0);

    for _ in 0..CAP / 10 {
        for i in 0..50 {
            q.push(()).unwrap();
            assert_eq!(q.len(), i + 1);
        }

        for i in 0..50 {
            q.pop().unwrap();
            assert_eq!(q.len(), 50 - i - 1);
        }
    }
    assert_eq!(q.len(), 0);

    for i in 0..CAP {
        q.push(()).unwrap();
        assert_eq!(q.len(), i + 1);
    }

    for _ in 0..CAP {
        q.pop().unwrap();
    }
    assert_eq!(q.len(), 0);

    Parallel::new()
        .add(|| {
            for _ in 0..COUNT {
                loop {
                    if let Ok(()) = q.pop() {
                        break;
                    }
                }
                let len = q.len();
                assert!(len <= CAP);
            }
        })
        .add(|| {
            for _ in 0..COUNT {
                while q.push(()).is_err() {}
                let len = q.len();
                assert!(len <= CAP);
            }
        })
        .run();

    assert_eq!(q.len(), 0);
}

#[test]
fn len_unbounded() {
    let q = ConcurrentQueue::<()>::unbounded();
    assert_eq!(q.len(), 0);

    for i in 0..1000 {
        q.push(()).unwrap();
        assert_eq!(q.len(), i + 1);
    }

    for _ in 0..1000 {
        q.pop().unwrap();
    }

    assert_eq!(q.len(), 0);
}

#[test]
fn close() {
    let q = ConcurrentQueue::<()>::bounded(2);
    assert_eq!(q.push(()), Ok(()));

    assert!(!q.is_closed());
    assert!(q.close());

    assert!(q.is_closed());
    assert!(!q.close());

    assert_eq!(q.push(()), Err(PushError::Closed(())));
    assert_eq!(q.pop(), Ok(()));
    assert_eq!(q.pop(), Err(PopError::Closed));
}

#[test]
fn close_unbounded() {
    let q = ConcurrentQueue::<()>::unbounded();
    assert_eq!(q.push(()), Ok(()));

    assert!(!q.is_closed());
    assert!(q.close());

    assert!(q.is_closed());
    assert!(!q.close());

    assert_eq!(q.push(()), Err(PushError::Closed(())));
    assert_eq!(q.pop(), Ok(()));
    assert_eq!(q.pop(), Err(PopError::Closed));
}

#[test]
fn spsc() {
    const COUNT: usize = if cfg!(miri) { 100 } else { 100_000 };

    let q = ConcurrentQueue::<()>::bounded(3);

    Parallel::new()
        .add(|| {
            for _ in 0..COUNT {
                loop {
                    if let Ok(()) = q.pop() {
                        break;
                    }
                }
            }
            assert!(q.pop().is_err());
        })
        .add(|| {
            for _ in 0..COUNT {
                while q.push(()).is_err() {}
            }
        })
        .run();
}

#[test]
fn drops() {
    const RUNS: usize = if cfg!(miri) { 10 } else { 100 };
    const STEPS: usize = if cfg!(miri) { 100 } else { 10_000 };

    static DROPS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, PartialEq)]
    struct DropCounter;

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::SeqCst);
        }
    }

    for _ in 0..RUNS {
        let steps = fastrand::usize(..STEPS);
        let additional = fastrand::usize(..50);

        DROPS.store(0, Ordering::SeqCst);
        let q = ConcurrentQueue::bounded(50);

        Parallel::new()
            .add(|| {
                for _ in 0..steps {
                    while q.pop().is_err() {}
                }
            })
            .add(|| {
                for _ in 0..steps {
                    while q.push(DropCounter).is_err() {
                        DROPS.fetch_sub(1, Ordering::SeqCst);
                    }
                }
            })
            .run();

        for _ in 0..additional {
            q.push(DropCounter).unwrap();
        }

        assert_eq!(DROPS.load(Ordering::SeqCst), steps);
        drop(q);
        assert_eq!(DROPS.load(Ordering::SeqCst), steps + additional);
    }
}

#[test]
fn linearizable() {
    const COUNT: usize = if cfg!(miri) { 500 } else { 25_000 };
    const THREADS: usize = 4;

    let q = ConcurrentQueue::bounded(THREADS);

    Parallel::new()
        .each(0..THREADS, |_| {
            for _ in 0..COUNT {
                while q.push(()).is_err() {}
                q.pop().unwrap();
            }
        })
        .run();
}
