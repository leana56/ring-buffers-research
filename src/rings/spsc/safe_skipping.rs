use super::{Sender, Receiver};
use crate::AnyPayload;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::mem::size_of;
use std::sync::Arc;

// =--------------------------------------------------------= //
// =-= Notes =-= //
// - The goal of this ring is to minimize latency via taking the tradeoff of data loss, safely, versus the unsafe method of possible UB coming from other implementations
// - Caveat: If the producer is meaningfully faster than the consumer, the consumer could miss *a lot* of data during the burst as the producer will outrun the consumer
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for SPSCSafeSkipping<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for SPSCSafeSkipping<T, RING_SIZE> {}

// =--------------------------------------------------------= //

#[repr(align(64))]
pub struct Slot<T: AnyPayload> {
    pub seq: AtomicUsize,
    _pad: [u8; 64 - size_of::<AtomicUsize>()],
    pub data: UnsafeCell<MaybeUninit<T>>,
}

impl<T: AnyPayload> Slot<T> {
    pub fn new(index: usize) -> Self {
        Self { seq: AtomicUsize::new(index), _pad: [0; 64 - size_of::<AtomicUsize>()], data: UnsafeCell::new(MaybeUninit::uninit()) }
    }
}

// =--------------------------------------------------------= //


#[repr(align(64))]
pub struct SPSCSafeSkipping<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    _pad: [u8; 64 - size_of::<AtomicUsize>()],
    pub ring: Box<[Slot<T>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> SPSCSafeSkipping<T, RING_SIZE> {
    pub fn new() -> (SPSCSafeSkippingWriter<T, RING_SIZE>, SPSCSafeSkippingReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            _pad: [0; 64 - size_of::<AtomicUsize>()],
            ring: (0..RING_SIZE).map(|i| Slot::new(i)).collect::<Vec<_>>().into_boxed_slice(),
        });

        (SPSCSafeSkippingWriter { inner: Arc::clone(&inner) }, SPSCSafeSkippingReader { inner })
    }
}

// =-= Writer =-= //

pub struct SPSCSafeSkippingWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPSCSafeSkipping<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPSCSafeSkippingWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for SPSCSafeSkippingWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        loop {
            let head = self.inner.head.load(Ordering::Relaxed);
            let slot = &self.inner.ring[head & (RING_SIZE - 1)];
            let seq  = slot.seq.load(Ordering::Acquire);
            
            if seq != head && seq + RING_SIZE > head {
                std::hint::spin_loop();
                continue;
            }

            unsafe { slot.data.get().write(MaybeUninit::new(item)); }
            slot.seq.store(head.wrapping_add(1), Ordering::Release);
            self.inner.head.store(head.wrapping_add(1), Ordering::Relaxed);
            return;
        }
    }
}

// =-= Reader =-= //

pub struct SPSCSafeSkippingReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPSCSafeSkipping<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPSCSafeSkippingReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for SPSCSafeSkippingReader<T, RING_SIZE> {
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
            *local_tail = head.saturating_sub(1);
        }
    }
}
