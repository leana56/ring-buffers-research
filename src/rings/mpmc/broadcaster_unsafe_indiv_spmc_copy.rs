use super::{Sender, Receiver};
use crate::AnyPayload;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::mem::size_of;
use std::sync::Arc;
use std::sync::Mutex;

// =--------------------------------------------------------= //
// =-= Notes =-= //
// - An implementation where each producer has it's own ring buffer and the consumers read from all individual ring buffers
// - This is *UNSAFE* and should not be used in production(!)
// - Copy is enforced for the payload
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload + Copy, const RING_SIZE: usize> Send for MPMCBroadcasterUnsafeIndivSPMCCopy<T, RING_SIZE> {}
unsafe impl<T: AnyPayload + Copy, const RING_SIZE: usize> Sync for MPMCBroadcasterUnsafeIndivSPMCCopy<T, RING_SIZE> {}

// =--------------------------------------------------------= //

pub struct IndivRing<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub ring: Box<[UnsafeCell<MaybeUninit<T>>]>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> IndivRing<T, RING_SIZE> {
    pub fn new() -> Self {
        Self { 
            head: AtomicUsize::new(0), 
            ring: (0..RING_SIZE).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect::<Vec<_>>().into_boxed_slice() 
        }
    }
}

// =--------------------------------------------------------= //

pub struct MPMCBroadcasterUnsafeIndivSPMCCopy<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub producer_rings: Box<[IndivRing<T, RING_SIZE>]>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> MPMCBroadcasterUnsafeIndivSPMCCopy<T, RING_SIZE> {
    pub fn new(producer_threads: usize) -> (MPMCBroadcasterUnsafeIndivSPMCCopyWriter<T, RING_SIZE>, MPMCBroadcasterUnsafeIndivSPMCCopyReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            producer_rings: (0..producer_threads).map(|i| IndivRing::new()).collect::<Vec<_>>().into_boxed_slice(),
        });

        let readers = (0..producer_threads).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>();

        (MPMCBroadcasterUnsafeIndivSPMCCopyWriter { inner: Arc::clone(&inner) }, MPMCBroadcasterUnsafeIndivSPMCCopyReader { readers, inner: Arc::clone(&inner) })
    }
}

// =-= Writer =-= //

pub struct MPMCBroadcasterUnsafeIndivSPMCCopyWriter<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub inner: Arc<MPMCBroadcasterUnsafeIndivSPMCCopy<T, RING_SIZE>>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Clone for MPMCBroadcasterUnsafeIndivSPMCCopyWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Sender<T> for MPMCBroadcasterUnsafeIndivSPMCCopyWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        let producers_ring = &self.inner.producer_rings[producer_id];

        let timer = minstant::Instant::now();
        loop {
            let head = producers_ring.head.load(Ordering::Acquire);

            if timer.elapsed().as_millis() > 1000 {
                return;
            }

            let slot = &producers_ring.ring[head & (RING_SIZE - 1)];
            unsafe { (*slot.get()).write(item); }
            producers_ring.head.store(head.wrapping_add(1), Ordering::Release);
            return;
        }
    }
}

// =-= Reader =-= //

pub struct MPMCBroadcasterUnsafeIndivSPMCCopyReader<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub readers: Vec<AtomicUsize>,
    pub inner: Arc<MPMCBroadcasterUnsafeIndivSPMCCopy<T, RING_SIZE>>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Clone for MPMCBroadcasterUnsafeIndivSPMCCopyReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        let mut readers = Vec::new();
        for producer_ring in self.inner.producer_rings.iter() {
            readers.push(AtomicUsize::new(producer_ring.head.load(Ordering::Acquire)));
        }

        Self { 
            readers, 
            inner: Arc::clone(&self.inner) 
        }
    }
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Receiver<T> for MPMCBroadcasterUnsafeIndivSPMCCopyReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            for (i, producer_ring) in self.inner.producer_rings.iter().enumerate() {
                let head = producer_ring.head.load(Ordering::Acquire);
                let tail = self.readers[i].load(Ordering::Acquire);

                if tail >= head {
                    std::hint::spin_loop();
                    continue;
                }

                let slot = &producer_ring.ring[tail & (RING_SIZE - 1)];
                let value = unsafe { (*slot.get()).assume_init_read() };
                self.readers[i].store(tail.wrapping_add(1), Ordering::Release);
                return value;
            }
        }
    }
}