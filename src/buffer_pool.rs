//! Buffer pool for reusing `BytesMut` allocations across read/write operations.
//!
//! See DEC-005 for the pool sizing and strategy rationale.

#![allow(clippy::unwrap_used)]
// PooledBuffer::inner is invariantly Some while live; Deref/DerefMut unwraps are safe.

use bytes::BytesMut;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

pub const DEFAULT_CAPACITY: usize = 8;
pub const DEFAULT_BUF_SIZE: usize = 256 * 1024; // 256KB per DEC-005

pub struct BufferPool {
    pool: Arc<Mutex<Vec<BytesMut>>>,
    buf_size: usize,
}

impl Clone for BufferPool {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            buf_size: self.buf_size,
        }
    }
}

impl BufferPool {
    pub fn new(capacity: usize, buf_size: usize) -> Self {
        let mut v = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            v.push(BytesMut::with_capacity(buf_size));
        }
        Self {
            pool: Arc::new(Mutex::new(v)),
            buf_size,
        }
    }

    pub fn acquire(&self) -> Option<PooledBuffer> {
        self.pool.lock().unwrap().pop().map(|buf| PooledBuffer {
            inner: Some(buf),
            pool: Arc::clone(&self.pool),
        })
    }

    pub fn available(&self) -> usize {
        self.pool.lock().unwrap().len()
    }
}

pub struct PooledBuffer {
    inner: Option<BytesMut>,
    pool: Arc<Mutex<Vec<BytesMut>>>,
}

impl Deref for PooledBuffer {
    type Target = BytesMut;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(mut buf) = self.inner.take() {
            buf.clear();
            // PoisonError means the holding thread already panicked; re-panic.
            self.pool.lock().unwrap().push(buf);
        }
    }
}
