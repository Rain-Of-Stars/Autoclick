pub struct DropGuard<F: FnOnce()> {
    cleanup: Option<F>,
}

impl<F: FnOnce()> DropGuard<F> {
    pub fn new(cleanup: F) -> Self {
        Self {
            cleanup: Some(cleanup),
        }
    }

    pub fn defuse(&mut self) {
        self.cleanup = None;
    }
}

impl<F: FnOnce()> Drop for DropGuard<F> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use super::DropGuard;

    #[test]
    fn drop_guard_runs_cleanup_on_drop() {
        let flag = Arc::new(AtomicBool::new(false));
        {
            let flag = flag.clone();
            let _guard = DropGuard::new(move || {
                flag.store(true, Ordering::SeqCst);
            });
        }
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn drop_guard_can_be_defused() {
        let flag = Arc::new(AtomicBool::new(false));
        {
            let flag = flag.clone();
            let mut guard = DropGuard::new(move || {
                flag.store(true, Ordering::SeqCst);
            });
            guard.defuse();
        }
        assert!(!flag.load(Ordering::SeqCst));
    }
}
