use std::sync::{Condvar, Mutex, OnceLock};

static SEMAPHORE: OnceLock<(Mutex<bool>, Condvar)> = OnceLock::new();

#[must_use]
pub fn wait_sequential() -> impl Drop {
    let (lock, cvar) = SEMAPHORE.get_or_init(|| (Mutex::new(false), Condvar::new()));
    let mut blocked = lock.lock().unwrap();
    while *blocked {
        blocked = cvar.wait(blocked).unwrap();
    }
    *blocked = true;

    struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            let (lock, cvar) = SEMAPHORE.get_or_init(|| (Mutex::new(false), Condvar::new()));
            let mut blocked = lock.lock().unwrap();
            *blocked = false;
            cvar.notify_one();
        }
    }

    Guard
}
