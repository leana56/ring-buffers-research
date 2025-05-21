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
// - Variant of the baseline "global_tail" however instead of using the global tail index it uses a local tail and updates the sequence counter with that
// - Lossy variant, data will be dropped if the consumer is too slow to keep up with the producer
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for MPSCLocalTailLossy<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for MPSCLocalTailLossy<T, RING_SIZE> {}

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

pub struct MPSCLocalTailLossy<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub ring: Box<[Slot<T>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> MPSCLocalTailLossy<T, RING_SIZE> {
    pub fn new() -> (MPSCLocalTailLossyWriter<T, RING_SIZE>, MPSCLocalTailLossyReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|i| Slot::new(i)).collect::<Vec<_>>().into_boxed_slice(),
        });

        (MPSCLocalTailLossyWriter { inner: Arc::clone(&inner) }, MPSCLocalTailLossyReader { inner })
    }
}


// =-= Writer =-= //

pub struct MPSCLocalTailLossyWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPSCLocalTailLossy<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPSCLocalTailLossyWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for MPSCLocalTailLossyWriter<T, RING_SIZE> {
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

pub struct MPSCLocalTailLossyReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPSCLocalTailLossy<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPSCLocalTailLossyReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for MPSCLocalTailLossyReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let slot = &self.inner.ring[*local_tail & (RING_SIZE - 1)];
            let seq = slot.seq.load(Ordering::Acquire);
            let expected = local_tail.wrapping_add(1);

            if seq == expected {
                let item = unsafe { slot.data.get().read().assume_init() };
                slot.seq.store(local_tail.wrapping_add(RING_SIZE), Ordering::Release);
                *local_tail = local_tail.wrapping_add(1);
                return item;
            }

            if seq == *local_tail {
                continue;
            }

            let head = self.inner.head.load(Ordering::Acquire);
            *local_tail = head.saturating_sub(1); // drops any data to catch up with the header
        }
    }
}