#![cfg(loom)]

use concurrent_queue::ConcurrentQueue;
use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::thread;
use std::sync::Arc;

fn run_test<F: Fn(ConcurrentQueue<usize>, usize) + Send + Sync + 'static>(f: F) {
    const LIMIT: usize = 500;

    loom::model(move || {
        // Run for single, bounded and unbounded.
        f(ConcurrentQueue::unbounded(), LIMIT);
        f(ConcurrentQueue::bounded(1), 1);
        f(ConcurrentQueue::bounded(LIMIT), LIMIT);
    });
}

#[test]
fn spsc() {
    run_test(|q, limit| {
        let q = Arc::new(q);

        let q1 = q.clone();
        let t1 = thread::spawn(move || {
            for i in 0..limit {
                while q1.push(i).is_err() {}
            }
        });

        let q2 = q.clone();
        let t2 = thread::spawn(move || {
            for i in 0..limit {
                loop {
                    if let Ok(x) = q.pop() {
                        assert_eq!(x, i);
                        break;
                    }
                }
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}

#[test]
fn mpmc() {
    run_test(|q, limit| {
        let pusher_threads = loom::MAX_THREADS / 2;
        let popper_threads = loom::MAX_THREADS - pusher_threads;

        let q = Arc::new(q);
        let v = (0..limit).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>();
        let v = Arc::new(v);

        let mut threads = Vec::new();

        for _ in 0..popper_threads {
            let q = q.clone();
            let v = v.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..limit {
                    let n = loop {
                        if let Ok(x) = q.pop() {
                            break x;
                        }
                    };
                    v[n].fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for _ in 0..pusher_threads {
            let q = q.clone();
            threads.push(thread::spawn(move || {
                for i in 0..limit {
                    while q.push(i).is_err() {}
                }
            }));
        }

        for t in threads {
            t.join().unwrap();
        }

        for c in Arc::try_unwrap(v).unwrap() {
            assert_eq!(c.load(Ordering::SeqCst), popper_threads);
        }
    });
}
