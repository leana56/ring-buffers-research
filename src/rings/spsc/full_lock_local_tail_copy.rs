use super::{Sender, Receiver};
use crate::AnyPayload;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Mutex;
use std::sync::Arc;

// =--------------------------------------------------------= //
// =-= Notes =-= //
// - An example of a ring buffer using a global lock instead of sequence counters to ward off data races (for show, not recommended)
// - Very unsafe implementation when T: !Copy
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload + Copy, const RING_SIZE: usize> Send for SPSCFullLockLocalTailCopy<T, RING_SIZE> {}
unsafe impl<T: AnyPayload + Copy, const RING_SIZE: usize> Sync for SPSCFullLockLocalTailCopy<T, RING_SIZE> {}

// =--------------------------------------------------------= //

pub struct SPSCFullLockLocalTailCopy<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub _lock: Mutex<()>,
    pub head: AtomicUsize,
    pub ring: Box<[UnsafeCell<MaybeUninit<T>>]>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> SPSCFullLockLocalTailCopy<T, RING_SIZE> {
    pub fn new() -> (SPSCFullLockLocalTailCopyWriter<T, RING_SIZE>, SPSCFullLockLocalTailCopyReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            _lock: Mutex::new(()),
            head: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect::<Vec<_>>().into_boxed_slice(),
        });

        (SPSCFullLockLocalTailCopyWriter { inner: Arc::clone(&inner) }, SPSCFullLockLocalTailCopyReader { inner })
    }
}


// =-= Writer =-= //

pub struct SPSCFullLockLocalTailCopyWriter<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub inner: Arc<SPSCFullLockLocalTailCopy<T, RING_SIZE>>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Clone for SPSCFullLockLocalTailCopyWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Sender<T> for SPSCFullLockLocalTailCopyWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        let header = self.inner.head.load(Ordering::Acquire);
        
        loop {
            let lock_result = self.inner._lock.try_lock();
            match lock_result {
                Ok(_lock) => {
                    unsafe { self.inner.ring[header & (RING_SIZE - 1)].get().write(MaybeUninit::new(item)); }
                    self.inner.head.store(header.wrapping_add(1), Ordering::Release);
                    return;
                }
                Err(_) => continue,
            }
        }
    }
}

// =-= Reader =-= //

pub struct SPSCFullLockLocalTailCopyReader<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub inner: Arc<SPSCFullLockLocalTailCopy<T, RING_SIZE>>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Clone for SPSCFullLockLocalTailCopyReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Receiver<T> for SPSCFullLockLocalTailCopyReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let head_now = self.inner.head.load(Ordering::Acquire);
            if *local_tail < head_now { 
                let lock_result = self.inner._lock.try_lock();
                match lock_result {
                    Ok(_lock) => {
                        let item = unsafe { self.inner.ring[*local_tail & (RING_SIZE - 1)].get().read().assume_init() };
                        *local_tail = local_tail.wrapping_add(1);
                        return item;
                    }
                    Err(_) => continue,
                }
            }
        }
    }
}