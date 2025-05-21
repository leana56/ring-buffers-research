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
// - Same as "global_tail" however it drops data if the consumer is too slow to keep up with the producer
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for MPSCGlobalTailLossy<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for MPSCGlobalTailLossy<T, RING_SIZE> {}

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

pub struct MPSCGlobalTailLossy<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub tail: AtomicUsize,
    pub ring: Box<[Slot<T>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> MPSCGlobalTailLossy<T, RING_SIZE> {
    pub fn new() -> (MPSCGlobalTailLossyWriter<T, RING_SIZE>, MPSCGlobalTailLossyReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|i| Slot::new(i)).collect::<Vec<_>>().into_boxed_slice(),
        });

        (MPSCGlobalTailLossyWriter { inner: Arc::clone(&inner) }, MPSCGlobalTailLossyReader { inner })
    }
}


// =-= Writer =-= //

pub struct MPSCGlobalTailLossyWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPSCGlobalTailLossy<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPSCGlobalTailLossyWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for MPSCGlobalTailLossyWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {

        let timer = minstant::Instant::now();
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
            let slot = &self.inner.ring[head & (RING_SIZE - 1)];
            let seq = slot.seq.load(Ordering::Acquire);
            
            if head == seq {
                match self.inner.head.compare_exchange_weak(head, head.wrapping_add(1), Ordering::AcqRel, Ordering::Relaxed) {
                    Ok(_) => {
                        unsafe { slot.data.get().write(MaybeUninit::new(item)); }
                        slot.seq.store(head.wrapping_add(1), Ordering::Release);
                        return;
                    },
                    Err(_) => continue,
                }
            }

            if timer.elapsed().as_millis() > 1000 {
                return;
            }

            std::hint::spin_loop();
        }
    }
}

// =-= Reader =-= //

pub struct MPSCGlobalTailLossyReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPSCGlobalTailLossy<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPSCGlobalTailLossyReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for MPSCGlobalTailLossyReader<T, RING_SIZE> {
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

            if seq == tail {
                continue;
            }

            let head = self.inner.head.load(Ordering::Acquire);
            self.inner.tail.store(head.saturating_sub(1), Ordering::Release); // drops any data to catch up with the header
        }
    }
}