use super::{Sender, Receiver};
use crate::AnyPayload;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Arc;

// =--------------------------------------------------------= //
// =-= Notes =-= //
// - A simple load balancer implementation that distributes the load across consumers dependent on the consumer's ability to consume at the moment (they decide when they want to contribute to the consumption)
// - Enforces the payload to implement the `Copy` trait (otherwise double free UB can occur - if you'd like to implement T: !Copy, the per-slot sequence counter is an easy fix)
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload + Copy, const RING_SIZE: usize> Send for SPMCLoadBalancerCopy<T, RING_SIZE> {}
unsafe impl<T: AnyPayload + Copy, const RING_SIZE: usize> Sync for SPMCLoadBalancerCopy<T, RING_SIZE> {}

// =--------------------------------------------------------= //

pub struct SPMCLoadBalancerCopy<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub tail: AtomicUsize,
    pub ring: Box<[UnsafeCell<MaybeUninit<T>>]>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> SPMCLoadBalancerCopy<T, RING_SIZE> {
    pub fn new() -> (SPMCLoadBalancerCopyWriter<T, RING_SIZE>, SPMCLoadBalancerCopyReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect::<Vec<_>>().into_boxed_slice(),
        });

        (SPMCLoadBalancerCopyWriter { inner: Arc::clone(&inner) }, SPMCLoadBalancerCopyReader { inner })
    }
}

// =-= Writer =-= //
// - Clone is only used for standardization across the other ring implementations, only 1 writer is allowed to exist (otherwise double-write UB can occur); enforce at runtime

pub struct SPMCLoadBalancerCopyWriter<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub inner: Arc<SPMCLoadBalancerCopy<T, RING_SIZE>>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Clone for SPMCLoadBalancerCopyWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Sender<T> for SPMCLoadBalancerCopyWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
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

pub struct SPMCLoadBalancerCopyReader<T: AnyPayload + Copy, const RING_SIZE: usize> {
    pub inner: Arc<SPMCLoadBalancerCopy<T, RING_SIZE>>,
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Clone for SPMCLoadBalancerCopyReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload + Copy, const RING_SIZE: usize> Receiver<T> for SPMCLoadBalancerCopyReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let tail = self.inner.tail.load(Ordering::Acquire);
            let head = self.inner.head.load(Ordering::Acquire);

            if tail >= head {
                std::hint::spin_loop();
                continue;
            }

            match self.inner.tail.compare_exchange_weak(tail, tail.wrapping_add(1), Ordering::AcqRel, Ordering::Relaxed) {
                Ok(_) => return unsafe { self.inner.ring[tail & (RING_SIZE - 1)].get().read().assume_init() },
                Err(_) => continue,
            }
        }
    }
}