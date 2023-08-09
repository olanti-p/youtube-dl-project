use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
struct WorkerControlHandleInner {
    stop_value: AtomicBool,
    pause_value: AtomicBool,
}

#[derive(Debug, Clone, Default)]
pub struct WorkerControlHandle {
    values: Arc<WorkerControlHandleInner>,
}

impl WorkerControlHandle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stop(&self) {
        self.values.stop_value.store(true, Ordering::Relaxed);
    }

    pub fn pause(&self) {
        self.values.pause_value.store(true, Ordering::Relaxed);
    }

    pub fn is_stopped(&self) -> bool {
        self.values.stop_value.load(Ordering::Relaxed)
    }

    pub fn is_paused(&self) -> bool {
        self.values.pause_value.load(Ordering::Relaxed)
    }
}
