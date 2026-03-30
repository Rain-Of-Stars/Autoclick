use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Default)]
pub struct ShutdownSignal {
    requested: Arc<AtomicBool>,
}

impl ShutdownSignal {
    pub fn request(&self) {
        self.requested.store(true, Ordering::SeqCst);
    }

    pub fn is_requested(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }

    pub fn sleep_cancelable(&self, duration: Duration) -> bool {
        if duration.is_zero() {
            return self.is_requested();
        }

        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            if self.is_requested() {
                return true;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            let step = remaining.min(Duration::from_millis(25));
            if !step.is_zero() {
                thread::sleep(step);
            }
        }
        self.is_requested()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use super::ShutdownSignal;

    #[test]
    fn sleep_cancelable_returns_when_signal_is_requested() {
        let signal = ShutdownSignal::default();
        let worker_signal = signal.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            worker_signal.request();
        });

        let start = Instant::now();
        let interrupted = signal.sleep_cancelable(Duration::from_millis(500));
        assert!(interrupted);
        assert!(start.elapsed() < Duration::from_millis(250));
    }
}
