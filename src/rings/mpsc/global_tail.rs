use super::{Sender, Receiver};
use crate::AnyPayload;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::RwLock;
use std::sync::Weak;
use std::sync::Arc;

// =--------------------------------------------------------= //
// =-= Notes =-= //
// - Baseline implementation using a global (atomic) tail index for the consumer
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for MPSCGlobalTail<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for MPSCGlobalTail<T, RING_SIZE> {}

// =--------------------------------------------------------= //

pub struct Slot<T: AnyPayload> {
    pub seq: AtomicUsize,
    pub data: UnsafeCell<MaybeUninit<T>>,
}

impl<T: AnyPayload> Slot<T> {
    pub fn new(index: usize) -> Self {
        Self { seq: AtomicUsize::new(index), data: UnsafeCell::new(MaybeUninit::uninit()) }
    }
}

// =--------------------------------------------------------= //

pub struct MPSCGlobalTail<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub tail: AtomicUsize,
    pub ring: Box<[Slot<T>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> MPSCGlobalTail<T, RING_SIZE> {
    pub fn new() -> (MPSCGlobalTailWriter<T, RING_SIZE>, MPSCGlobalTailReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|i| Slot::new(i)).collect::<Vec<_>>().into_boxed_slice(),
        });

        (MPSCGlobalTailWriter { inner: Arc::clone(&inner) }, MPSCGlobalTailReader { inner })
    }
}


// =-= Writer =-= //

pub struct MPSCGlobalTailWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPSCGlobalTail<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPSCGlobalTailWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for MPSCGlobalTailWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        let timer = minstant::Instant::now();
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
            let tail = self.inner.tail.load(Ordering::Acquire);

            if timer.elapsed().as_millis() > 1000 {
                return;
            }

            if head.wrapping_sub(tail) >= RING_SIZE {
                std::hint::spin_loop();
                continue;
            }

            let slot = &self.inner.ring[head & (RING_SIZE - 1)];
            if head == slot.seq.load(Ordering::Acquire) {
                match self.inner.head.compare_exchange_weak(head, head.wrapping_add(1), Ordering::AcqRel, Ordering::Relaxed) {
                    Ok(_) => {
                        unsafe { slot.data.get().write(MaybeUninit::new(item)); }
                        slot.seq.store(head.wrapping_add(1), Ordering::Release);
                        return;
                    },
                    Err(_) => continue,
                }
            }
        }
    }
}

// =-= Reader =-= //

pub struct MPSCGlobalTailReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPSCGlobalTail<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPSCGlobalTailReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for MPSCGlobalTailReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let tail = self.inner.tail.load(Ordering::Relaxed);
            let slot = &self.inner.ring[tail & (RING_SIZE - 1)];
            let seq = slot.seq.load(Ordering::Acquire);
            let expected = tail.wrapping_add(1);

            if seq == expected {
                let item = unsafe { slot.data.get().read().assume_init() };
                slot.seq.store(tail.wrapping_add(RING_SIZE), Ordering::Release);
                self.inner.tail.store(tail.wrapping_add(1), Ordering::Release);
                return item;
            }
        }
    }
}