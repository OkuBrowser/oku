#[cfg(feature = "deadlock_detection")]
pub(crate) fn spawn_deadlock_checker() {
    use log::{error, info};
    use parking_lot::deadlock;
    use std::thread;
    use std::time::Duration;

    // Create a background thread which checks for deadlocks every 10s
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(10));
            let deadlocks = deadlock::check_deadlock();
            if deadlocks.is_empty() {
                info!("# No deadlock");
                continue;
            }

            eprintln!("# {} deadlocks detected", deadlocks.len());
            for (i, threads) in deadlocks.iter().enumerate() {
                error!("Deadlock #{}", i);
                for t in threads {
                    error!("Thread Id {:#?}\n, {:#?}", t.thread_id(), t.backtrace());
                }
            }
        }
    });
}

/// Returns the default generation number for FUSE replies.
///
/// We return 0 by default to avoid invalidating kernel dentries during lookup
/// re-validation. Returning a different/random generation number on every lookup
/// would cause the kernel to think the inode was replaced.
///
/// NOTE: Unique generation numbers are only required if exporting the FUSE filesystem
/// over NFS. For NFS support, filesystems should explicitly track generation numbers
/// and populate the `generation` field of `FileAttribute`.
pub(crate) fn get_default_generation() -> u64 {
    0
}
