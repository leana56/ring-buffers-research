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
// - Simple (no optimizations) work-stealing / load balancing implementation using the classic seq / disruptor pattern as seen in other implementations
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for MPMCLoadBalancer<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for MPMCLoadBalancer<T, RING_SIZE> {}

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

pub struct MPMCLoadBalancer<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub tail: AtomicUsize,
    pub ring: Box<[Slot<T>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> MPMCLoadBalancer<T, RING_SIZE> {
    pub fn new() -> (MPMCLoadBalancerWriter<T, RING_SIZE>, MPMCLoadBalancerReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|i| Slot::new(i)).collect::<Vec<_>>().into_boxed_slice(),
        });

        (MPMCLoadBalancerWriter { inner: Arc::clone(&inner) }, MPMCLoadBalancerReader { inner })
    }
}

// =-= Writer =-= //

pub struct MPMCLoadBalancerWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPMCLoadBalancer<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPMCLoadBalancerWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for MPMCLoadBalancerWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {

        let timer = minstant::Instant::now();
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
            let slot = &self.inner.ring[head & (RING_SIZE - 1)];
            let seq = slot.seq.load(Ordering::Acquire);

            if seq == head && self.inner.head.compare_exchange_weak(head, head.wrapping_add(1), Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                // if head >= RING_SIZE && std::mem::needs_drop::<T>() {
                //     unsafe { (*slot.data.get()).assume_init_drop(); }
                // }

                unsafe { slot.data.get().write(MaybeUninit::new(item)); }
                slot.seq.store(head.wrapping_add(1), Ordering::Release);
                return;
            }

            if timer.elapsed().as_millis() > 1000 {
                return;
            }

            std::hint::spin_loop();
        }
    }
}

// =-= Reader =-= //

pub struct MPMCLoadBalancerReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPMCLoadBalancer<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPMCLoadBalancerReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for MPMCLoadBalancerReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let tail = self.inner.tail.load(Ordering::Acquire);
            let slot = &self.inner.ring[tail & (RING_SIZE - 1)];
            let seq = slot.seq.load(Ordering::Acquire);
            let expected = tail + 1;

            if seq == expected {
                match self.inner.tail.compare_exchange_weak(tail, tail.wrapping_add(1), Ordering::AcqRel, Ordering::Relaxed) {
                    Ok(_) => {
                        let item = unsafe { (*slot.data.get()).assume_init_read() };
                        slot.seq.store(tail.wrapping_add(RING_SIZE), Ordering::Release);
                        return item;
                    },
                    Err(_) => continue,
                }
            }

            std::hint::spin_loop();
        }
    }
}