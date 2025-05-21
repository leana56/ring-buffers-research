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
// - Each producer has their own ring they push to and the singular consumer reads from all the rings
// - Since it's one consumer we can use a global tail index (respective to the producer's ring)
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for MPSCIndivSPSCGroup<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for MPSCIndivSPSCGroup<T, RING_SIZE> {}

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

pub struct IndivRing<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub tail: AtomicUsize,
    pub ring: Box<[Slot<T>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> IndivRing<T, RING_SIZE> {
    pub fn new() -> Self {
        Self { 
            head: AtomicUsize::new(0), 
            tail: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|i| Slot::new(i)).collect::<Vec<_>>().into_boxed_slice() 
        }
    }
}

// =--------------------------------------------------------= //

pub struct MPSCIndivSPSCGroup<T: AnyPayload, const RING_SIZE: usize> {
    pub rings: Vec<IndivRing<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> MPSCIndivSPSCGroup<T, RING_SIZE> {
    pub fn new(producer_threads: usize) -> (MPSCIndivSPSCGroupWriter<T, RING_SIZE>, MPSCIndivSPSCGroupReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            rings: (0..producer_threads).map(|_| IndivRing::new()).collect::<Vec<_>>(),
        });

        (MPSCIndivSPSCGroupWriter { inner: Arc::clone(&inner) }, MPSCIndivSPSCGroupReader { inner })
    }
}


// =-= Writer =-= //

pub struct MPSCIndivSPSCGroupWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPSCIndivSPSCGroup<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPSCIndivSPSCGroupWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for MPSCIndivSPSCGroupWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        let producers_ring = &self.inner.rings[producer_id];

        let timer = minstant::Instant::now();
        loop {
            let head = producers_ring.head.load(Ordering::Acquire);
            let tail = producers_ring.tail.load(Ordering::Acquire);

            if timer.elapsed().as_millis() > 1000 {
                return;
            }

            if head.wrapping_sub(tail) >= RING_SIZE {
                std::hint::spin_loop();
                continue;
            }

            let slot = &producers_ring.ring[head & (RING_SIZE - 1)];
            if head == slot.seq.load(Ordering::Acquire) {
                unsafe { slot.data.get().write(MaybeUninit::new(item)); }
                slot.seq.store(head.wrapping_add(1), Ordering::Release);
                producers_ring.head.store(head.wrapping_add(1), Ordering::Release);
                return;
            }
        }
    }
}

// =-= Reader =-= //

pub struct MPSCIndivSPSCGroupReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPSCIndivSPSCGroup<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPSCIndivSPSCGroupReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for MPSCIndivSPSCGroupReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            for producer_ring in &self.inner.rings {
                let head = producer_ring.head.load(Ordering::Acquire);
                let tail = producer_ring.tail.load(Ordering::Acquire);

                if tail >= head {
                    continue;
                }

                let slot = &producer_ring.ring[tail & (RING_SIZE - 1)];
                let seq = slot.seq.load(Ordering::Acquire);
                let expected = tail + 1;
    
                if seq == expected {
                    let item = unsafe { slot.data.get().read().assume_init() };
                    slot.seq.store(tail.wrapping_add(RING_SIZE), Ordering::Release);
                    producer_ring.tail.store(tail.wrapping_add(1), Ordering::Release);
                    return item;
                }
            }
        }
    }
}