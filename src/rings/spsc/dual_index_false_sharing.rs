use super::{Sender, Receiver};
use crate::AnyPayload;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Arc;

// =--------------------------------------------------------= //
// =-= Notes =-= //
// - A correct baseline implementation, when no assumptions can be or are desired to be made about the producer and consumer's relative characteristics
// - Good starting point to base other implementations off of if these guarantees are desired
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for SPSCDualIndexFalseSharing<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for SPSCDualIndexFalseSharing<T, RING_SIZE> {}

// =--------------------------------------------------------= //

pub struct SPSCDualIndexFalseSharing<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub tail: AtomicUsize,
    pub ring: Box<[UnsafeCell<MaybeUninit<T>>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> SPSCDualIndexFalseSharing<T, RING_SIZE> {
    pub fn new() -> (SPSCDualIndexFalseSharingWriter<T, RING_SIZE>, SPSCDualIndexFalseSharingReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect::<Vec<_>>().into_boxed_slice(),
        });

        (SPSCDualIndexFalseSharingWriter { inner: Arc::clone(&inner) }, SPSCDualIndexFalseSharingReader { inner })
    }
}


// =-= Writer =-= //

pub struct SPSCDualIndexFalseSharingWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPSCDualIndexFalseSharing<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPSCDualIndexFalseSharingWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for SPSCDualIndexFalseSharingWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        loop {
            let head = self.inner.head.load(Ordering::Relaxed);
            let tail = self.inner.tail.load(Ordering::Acquire);
            
            if head.wrapping_sub(tail) >= RING_SIZE {
                std::hint::spin_loop();
                continue;
            }

            unsafe { self.inner.ring[head & (RING_SIZE - 1)].get().write(MaybeUninit::new(item)); }
            self.inner.head.store(head.wrapping_add(1), Ordering::Release);
            return;
        }
    }
}


// =-= Reader =-= //

pub struct SPSCDualIndexFalseSharingReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPSCDualIndexFalseSharing<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPSCDualIndexFalseSharingReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for SPSCDualIndexFalseSharingReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
            let tail = self.inner.tail.load(Ordering::Relaxed);
            
            if tail >= head {
                std::hint::spin_loop();
                continue;
            }

            let item = unsafe { self.inner.ring[tail & (RING_SIZE - 1)].get().read().assume_init() };
            self.inner.tail.store(tail.wrapping_add(1), Ordering::Release);
            return item;
        }
    }
}
