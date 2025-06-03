use crate::error_handler::BufferManager;
use crate::error_handler::types::{ErrorEvent, LogEvent};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, interval};

// WARNING: The use of async Mutex for buffer management is appropriate for most async Rust workloads,
// but may become a bottleneck under high concurrency. For high-throughput or low-latency systems,
// consider using lock-free or sharded data structures (e.g., dashmap, crossbeam) to reduce contention.
pub struct InMemoryBufferManager {
    info_buffer: Mutex<VecDeque<LogEvent>>,
    error_buffer: Mutex<VecDeque<ErrorEvent>>,
    max_size: usize,
}

impl InMemoryBufferManager {
    pub fn new(max_size: usize) -> Arc<Self> {
        Arc::new(Self {
            info_buffer: Mutex::new(VecDeque::with_capacity(max_size)),
            error_buffer: Mutex::new(VecDeque::with_capacity(max_size)),
            max_size,
        })
    }

    pub fn spawn_flusher(self: Arc<Self>, flush_interval_secs: u64) {
        let this = self.clone();
        tokio::spawn(async move {
            let mut intv = interval(Duration::from_secs(flush_interval_secs));
            loop {
                intv.tick().await;
                this.flush().await;
            }
        });
    }

    pub async fn flush(&self) {
        // Placeholder: persist or rotate buffer contents as needed
        let mut info = self.info_buffer.lock().await;
        let mut error = self.error_buffer.lock().await;
        info.clear();
        error.clear();
        // In production, persist to file/db before clearing
    }
}

#[async_trait::async_trait]
impl BufferManager for InMemoryBufferManager {
    async fn buffer_info(&self, event: &LogEvent) {
        let mut buf = self.info_buffer.lock().await;
        if buf.len() == self.max_size {
            buf.pop_front();
        }
        buf.push_back(event.clone());
    }
    async fn buffer_warning(&self, event: &ErrorEvent) {
        self.buffer_error(event).await;
    }
    async fn buffer_error(&self, event: &ErrorEvent) {
        let mut buf = self.error_buffer.lock().await;
        if buf.len() == self.max_size {
            buf.pop_front();
        }
        buf.push_back(event.clone());
    }
    async fn snapshot(&self) -> (Vec<LogEvent>, Vec<ErrorEvent>) {
        let info = self.info_buffer.lock().await;
        let error = self.error_buffer.lock().await;
        (
            info.iter().cloned().collect(),
            error.iter().cloned().collect(),
        )
    }
}
