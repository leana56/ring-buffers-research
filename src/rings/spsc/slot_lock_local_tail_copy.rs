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
// - An example of a ring buffer using locks (respective to the slot) instead of sequence counters to ward off data races (for show, not recommended)
// - Very unsafe implementation when T: !Copy
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload + Copy, const RING_SIZE: usize> Send for SPSCSlotLockLocalTailCopy<T, RING_SIZE> {}
unsafe impl<T: AnyPayload + Copy, const RING_SIZE: usize> Sync for SPSCSlotLockLocalTailCopy<T, RING_SIZE> {}

// =--------------------------------------------------------= //

pub struct LockSlot<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub _lock: Mutex<()>,
    pub data: UnsafeCell<MaybeUninit<T>>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> LockSlot<T, RING_SIZE> {
    pub fn new() -> Self {
        Self { _lock: Mutex::new(()), data: UnsafeCell::new(MaybeUninit::uninit()) }
    }
}

// =--------------------------------------------------------= //

pub struct SPSCSlotLockLocalTailCopy<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub ring: Box<[LockSlot<T, RING_SIZE>]>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> SPSCSlotLockLocalTailCopy<T, RING_SIZE> {
    pub fn new() -> (SPSCSlotLockLocalTailCopyWriter<T, RING_SIZE>, SPSCSlotLockLocalTailCopyReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|_| LockSlot::new()).collect::<Vec<_>>().into_boxed_slice(),
        });

        (SPSCSlotLockLocalTailCopyWriter { inner: Arc::clone(&inner) }, SPSCSlotLockLocalTailCopyReader { inner })
    }
}


// =-= Writer =-= //

pub struct SPSCSlotLockLocalTailCopyWriter<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub inner: Arc<SPSCSlotLockLocalTailCopy<T, RING_SIZE>>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Clone for SPSCSlotLockLocalTailCopyWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Sender<T> for SPSCSlotLockLocalTailCopyWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        let head = self.inner.head.load(Ordering::Acquire);
        let slot = &self.inner.ring[head & (RING_SIZE - 1)];
        
        loop {
            let lock_result = slot._lock.try_lock();
            match lock_result {
                Ok(_lock) => {
                    if head >= RING_SIZE && std::mem::needs_drop::<T>() {
                        unsafe { (*slot.data.get()).assume_init_drop(); }
                    }

                    unsafe { (slot.data.get()).write(MaybeUninit::new(item)); }
                    self.inner.head.store(head.wrapping_add(1), Ordering::Release);
                    return;
                }
                Err(_) => std::hint::spin_loop(),
            }
        }
    }
}

// =-= Reader =-= //

pub struct SPSCSlotLockLocalTailCopyReader<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub inner: Arc<SPSCSlotLockLocalTailCopy<T, RING_SIZE>>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Clone for SPSCSlotLockLocalTailCopyReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Receiver<T> for SPSCSlotLockLocalTailCopyReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let head_now = self.inner.head.load(Ordering::Acquire);
            if *local_tail < head_now { 
                let slot = &self.inner.ring[*local_tail & (RING_SIZE - 1)];
                let lock_result = slot._lock.try_lock();
                match lock_result {
                    Ok(_lock) => {
                        let item = unsafe { slot.data.get().read().assume_init() };
                        *local_tail = local_tail.wrapping_add(1);
                        return item;
                    }
                    Err(_) => continue,
                }
            }
        }
    }
}