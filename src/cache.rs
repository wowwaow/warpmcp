use lru::LruCache;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CacheMetrics {
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    evictions: Arc<AtomicU64>,
}

impl CacheMetrics {
    pub fn new() -> Self {
        Self {
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            evictions: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
            self.evictions.load(Ordering::Relaxed),
        )
    }
}

struct CacheEntry {
    value: Value,
    expires_at: Instant,
}

pub struct ResponseCache {
    cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    ttl: Duration,
    metrics: CacheMetrics,
}

impl ResponseCache {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity.try_into().unwrap()))),
            ttl,
            metrics: CacheMetrics::new(),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Value> {
        let mut cache = self.cache.write().await;
        
        match cache.get(key) {
            Some(entry) if entry.expires_at > Instant::now() => {
                self.metrics.record_hit();
                Some(entry.value.clone())
            }
            Some(_) => {
                // Entry expired
                cache.pop(key);
                self.metrics.record_eviction();
                self.metrics.record_miss();
                None
            }
            None => {
                self.metrics.record_miss();
                None
            }
        }
    }

    pub async fn set(&self, key: String, value: Value) {
        let mut cache = self.cache.write().await;
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        };
        
        if cache.put(key, entry).is_some() {
            self.metrics.record_eviction();
        }
    }

    pub fn get_metrics(&self) -> &CacheMetrics {
        &self.metrics
    }
}

// Output buffer for batching responses
pub struct ResponseBuffer {
    buffer: Vec<String>,
    max_size: usize,
    current_size: usize,
}

impl ResponseBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(32), // Start with reasonable capacity
            max_size,
            current_size: 0,
        }
    }

    pub fn add(&mut self, response: String) -> bool {
        let response_size = response.len();
        if self.current_size + response_size > self.max_size {
            return false;
        }
        
        self.current_size += response_size;
        self.buffer.push(response);
        true
    }

    pub fn should_flush(&self) -> bool {
        self.current_size >= self.max_size / 2 || self.buffer.len() >= 50
    }

    pub fn take_buffer(&mut self) -> Vec<String> {
        let buffer = std::mem::take(&mut self.buffer);
        self.current_size = 0;
        buffer
    }
}
