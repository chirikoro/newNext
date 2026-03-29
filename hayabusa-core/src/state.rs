use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application-wide shared state container.
///
/// Allows storing and retrieving typed state that persists across requests.
#[derive(Clone, Default)]
pub struct AppState {
    inner: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a typed value into the state store.
    pub async fn insert<T: Send + Sync + 'static>(&self, value: T) {
        let mut map = self.inner.write().await;
        map.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Get a clone of a typed value from the state store.
    pub async fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        let map = self.inner.read().await;
        map.get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
            .cloned()
    }

    /// Check if a typed value exists in the state store.
    pub async fn contains<T: Send + Sync + 'static>(&self) -> bool {
        let map = self.inner.read().await;
        map.contains_key(&TypeId::of::<T>())
    }

    /// Remove a typed value from the state store.
    pub async fn remove<T: Send + Sync + 'static>(&self) -> Option<T> {
        let mut map = self.inner.write().await;
        map.remove(&TypeId::of::<T>())
            .and_then(|v| v.downcast::<T>().ok())
            .map(|v| *v)
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish()
    }
}
