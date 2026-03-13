use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use crate::Store;

/// A simple connection pool for Store instances.
///
/// With WAL mode enabled, multiple readers can operate concurrently.
/// The pool hands out connections and returns them automatically via RAII.
pub struct StorePool {
    conns: Arc<Mutex<Vec<Store>>>,
    path: String,
}

impl StorePool {
    /// Create a pool with `size` connections to the database at `path`.
    pub fn open(path: &str, size: usize) -> Result<Self, rusqlite::Error> {
        let mut conns = Vec::with_capacity(size);
        for _ in 0..size {
            conns.push(Store::open(path)?);
        }
        Ok(Self {
            conns: Arc::new(Mutex::new(conns)),
            path: path.to_string(),
        })
    }

    /// Get a connection from the pool.
    /// If the pool is empty, creates a new connection on-the-fly.
    pub fn get(&self) -> Result<PooledStore, rusqlite::Error> {
        let store = {
            let mut pool = self.conns.lock().unwrap();
            pool.pop()
        };
        let store = match store {
            Some(s) => s,
            None => Store::open(&self.path)?,
        };
        Ok(PooledStore {
            store: Some(store),
            pool: self.conns.clone(),
        })
    }
}

impl Clone for StorePool {
    fn clone(&self) -> Self {
        Self {
            conns: self.conns.clone(),
            path: self.path.clone(),
        }
    }
}

/// A connection checked out from the pool. Returns to pool on drop.
pub struct PooledStore {
    store: Option<Store>,
    pool: Arc<Mutex<Vec<Store>>>,
}

impl Deref for PooledStore {
    type Target = Store;
    fn deref(&self) -> &Store {
        self.store.as_ref().unwrap()
    }
}

impl DerefMut for PooledStore {
    fn deref_mut(&mut self) -> &mut Store {
        self.store.as_mut().unwrap()
    }
}

impl Drop for PooledStore {
    fn drop(&mut self) {
        if let Some(store) = self.store.take() {
            let mut pool = self.pool.lock().unwrap();
            pool.push(store);
        }
    }
}
