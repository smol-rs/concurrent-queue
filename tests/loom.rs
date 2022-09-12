#![cfg(loom)]

use concurrent_queue::ConcurrentQueue;
use loom::sync::Arc;
use loom::thread;

/// Wrapper to run tests on all three queues.
fn run_test<F: Fn(ConcurrentQueue<usize>, usize) + Send + Sync + Clone + 'static>(f: F) {
    const LIMIT: usize = 2;

    let fc = f.clone();
    loom::model(move || {
        fc(ConcurrentQueue::bounded(1), 1);
    });

    let fc = f.clone();
    loom::model(move || {
        fc(ConcurrentQueue::bounded(LIMIT), LIMIT);
    });

    return;

    loom::model(move || {
        f(ConcurrentQueue::unbounded(), LIMIT);
    });
}

#[test]
fn spsc() {
    run_test(|q, limit| {
        let q = Arc::new(q);
        let q1 = q.clone();

        // Spawn a thread that pushes items into the queue.
        thread::spawn(move || {
            for i in 0..limit {
                q1.push(i).unwrap();
            }
        });

        // Spawn a thread that pops items from the queue.
        thread::spawn(move || {
            for i in 0..limit {
                loop {
                    if let Ok(x) = q.pop() {
                        assert_eq!(x, i);
                        break;
                    }

                    thread::yield_now();
                }
            }
        });
    });
}