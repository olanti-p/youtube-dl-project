use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct GenericStopHandle {
    value: Arc<AtomicBool>,
}

impl Default for GenericStopHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericStopHandle {
    pub fn new() -> Self {
        Self {
            value: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn stop(&self) {
        self.value.store(true, Ordering::Relaxed);
    }

    pub fn is_stopped(&self) -> bool {
        self.value.load(Ordering::Relaxed)
    }
}
