# async-mutex

[![Build](https://github.com/stjepang/async-mutex/workflows/Build%20and%20test/badge.svg)](
https://github.com/stjepang/async-mutex/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/stjepang/async-mutex)
[![Cargo](https://img.shields.io/crates/v/async-mutex.svg)](
https://crates.io/crates/async-mutex)
[![Documentation](https://docs.rs/async-mutex/badge.svg)](
https://docs.rs/async-mutex)

An async mutex.

The locking mechanism uses eventual fairness to ensure locking will be fair on average without
sacrificing performance. This is done by forcing a fair lock whenever a lock operation is
starved for longer than 0.5 milliseconds.

## Examples

```rust
use async_mutex::Mutex;
use smol::Task;
use std::sync::Arc;

let m = Arc::new(Mutex::new(0));
let mut tasks = vec![];

for _ in 0..10 {
    let m = m.clone();
    tasks.push(Task::spawn(async move {
        *m.lock().await += 1;
    }));
}

for t in tasks {
    t.await;
}
assert_eq!(*m.lock().await, 10);
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
