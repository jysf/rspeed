#![allow(clippy::unwrap_used, clippy::expect_used)]

use rspeed::buffer_pool::{BufferPool, DEFAULT_BUF_SIZE, DEFAULT_CAPACITY};

#[test]
fn capacity_respected() {
    let pool = BufferPool::new(DEFAULT_CAPACITY, DEFAULT_BUF_SIZE);
    let handles: Vec<_> = (0..DEFAULT_CAPACITY).map(|_| pool.acquire()).collect();
    assert!(handles.iter().all(|h| h.is_some()));
    assert!(pool.acquire().is_none());
}

#[test]
fn buffer_returns_on_drop() {
    let pool = BufferPool::new(1, DEFAULT_BUF_SIZE);
    let handle = pool.acquire();
    assert!(handle.is_some());
    drop(handle);
    let handle2 = pool.acquire();
    assert!(handle2.is_some());
    drop(handle2);
    assert_eq!(pool.available(), 1);
}

#[test]
fn buffer_cleared_on_return() {
    let pool = BufferPool::new(1, DEFAULT_BUF_SIZE);
    let mut buf = pool.acquire().unwrap();
    buf.extend_from_slice(b"hello");
    drop(buf);
    let buf = pool.acquire().unwrap();
    assert_eq!(buf.len(), 0);
    assert!(buf.capacity() >= DEFAULT_BUF_SIZE);
}

#[test]
fn buffer_size_at_least_default() {
    let pool = BufferPool::new(DEFAULT_CAPACITY, DEFAULT_BUF_SIZE);
    let mut buf = pool.acquire().unwrap();
    assert!(buf.capacity() >= DEFAULT_BUF_SIZE);
    assert_eq!(buf.len(), 0);
    buf.extend_from_slice(&[0u8; 1024]);
    assert_eq!(buf.len(), 1024);
}

#[test]
fn available_tracks_acquire_and_drop() {
    let pool = BufferPool::new(4, 4096);
    assert_eq!(pool.available(), 4);
    let h1 = pool.acquire();
    let h2 = pool.acquire();
    assert_eq!(pool.available(), 2);
    drop(h1);
    assert_eq!(pool.available(), 3);
    drop(h2);
    assert_eq!(pool.available(), 4);
}

#[test]
fn deref_mut_allows_extend() {
    let pool = BufferPool::new(1, DEFAULT_BUF_SIZE);
    let mut buf = pool.acquire().unwrap();
    buf.extend_from_slice(b"test");
    assert_eq!(buf.len(), 4);
}

#[test]
fn clone_shares_pool() {
    let pool = BufferPool::new(2, DEFAULT_BUF_SIZE);
    let pool2 = pool.clone();
    let h1 = pool.acquire();
    let h2 = pool2.acquire();
    assert_eq!(pool.available(), 0);
    assert_eq!(pool2.available(), 0);
    drop(h1);
    drop(h2);
    assert_eq!(pool.available(), 2);
    assert_eq!(pool2.available(), 2);
}
