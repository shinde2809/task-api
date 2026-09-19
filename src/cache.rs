/// In-memory per-user task cache using DashMap.
///
/// NOTE: This is an in-memory cache. It does not persist across server restarts
/// and does not replicate across multiple instances. For production, replace with
/// Redis using the `redis` or `fred` crate. The cache.hit semantics and
/// invalidation logic remain identical — only the backing store changes.
use dashmap::DashMap;
use std::sync::Arc;

use crate::models::CachedTasks;

#[derive(Clone)]
pub struct TaskCache {
    inner: Arc<DashMap<String, CachedTasks>>,
}

impl TaskCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    /// Returns the cached value for `user_id`, if present.
    pub fn get(&self, user_id: &str) -> Option<CachedTasks> {
        self.inner.get(user_id).map(|v| v.clone())
    }

    /// Stores a value for `user_id`.
    pub fn set(&self, user_id: &str, data: CachedTasks) {
        self.inner.insert(user_id.to_string(), data);
    }

    /// Removes the cached value for `user_id` (call after task assignment).
    pub fn invalidate(&self, user_id: &str) {
        self.inner.remove(user_id);
    }
}

impl Default for TaskCache {
    fn default() -> Self {
        Self::new()
    }
}
