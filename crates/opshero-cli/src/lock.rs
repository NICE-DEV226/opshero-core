//! Advisory file locks (`flock`) with a timeout, built on the standard library.
//!
//! Used to serialise the two read-modify-write sequences that several recorders
//! may run at once: the private git index and the evidence chain. The lock is
//! released when the guard is dropped, and by the operating system if the
//! process dies, so a crashed recorder can never leave a stale lock behind.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::time::{Duration, Instant};

pub struct Lock {
    _file: File, // closing the file releases the lock
}

/// Take an exclusive lock on `path` (created if missing), waiting at most
/// `timeout`. `None` means the lock could not be had: the caller decides
/// whether to proceed without it or to fall back.
pub fn acquire(path: &Path, timeout: Duration) -> Option<Lock> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .ok()?;
    let deadline = Instant::now() + timeout;
    loop {
        match file.try_lock() {
            Ok(()) => return Some(Lock { _file: file }),
            Err(TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(TryLockError::Error(_)) => return None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn a_held_lock_blocks_others_until_it_is_dropped() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("l");
        let first = acquire(&p, Duration::from_millis(100)).expect("free lock");
        let started = Instant::now();
        assert!(
            acquire(&p, Duration::from_millis(60)).is_none(),
            "must time out"
        );
        assert!(
            started.elapsed() >= Duration::from_millis(55),
            "must actually wait"
        );
        drop(first);
        assert!(
            acquire(&p, Duration::from_millis(100)).is_some(),
            "free again after drop"
        );
    }

    #[test]
    fn a_missing_directory_gives_none_not_a_panic() {
        assert!(
            acquire(
                Path::new("/nonexistent-dir-xyz/lock"),
                Duration::from_millis(10)
            )
            .is_none()
        );
    }

    #[test]
    fn waiting_threads_all_get_the_lock_one_at_a_time() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let d = TempDir::new().unwrap();
        let p = Arc::new(d.path().join("l"));
        let inside = Arc::new(AtomicUsize::new(0));
        let max_inside = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let (p, inside, max_inside) = (p.clone(), inside.clone(), max_inside.clone());
                std::thread::spawn(move || {
                    let g = acquire(&p, Duration::from_secs(5)).expect("must eventually get it");
                    let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
                    max_inside.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(5));
                    inside.fetch_sub(1, Ordering::SeqCst);
                    drop(g);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(
            max_inside.load(Ordering::SeqCst),
            1,
            "mutual exclusion violated"
        );
    }
}
