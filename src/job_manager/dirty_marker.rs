use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct DirtyMarker {
    value: Arc<AtomicBool>,
}

impl DirtyMarker {
    pub fn new() -> Self {
        Self {
            value: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn mark_dirty(&self) {
        self.value.store(true, Ordering::Relaxed);
    }

    pub fn mark_clean(&self) {
        self.value.store(false, Ordering::Relaxed);
    }

    pub fn is_dirty(&self) -> bool {
        self.value.load(Ordering::Relaxed)
    }
}
