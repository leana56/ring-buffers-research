use super::{Sender, Receiver};
use crate::AnyPayload;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Arc;

// =--------------------------------------------------------= //
// =-= Notes =-= //
// - Same as `broadcaster_unsafe_local_tails` but instead of distinct Sender / Receiver structs, it's a single struct that is wrapped in an Arc
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for SPMCBroadcasterUnsafeLocalTailsOutsideArc<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for SPMCBroadcasterUnsafeLocalTailsOutsideArc<T, RING_SIZE> {}

// =--------------------------------------------------------= //

pub struct SPMCBroadcasterUnsafeLocalTailsOutsideArc<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub ring: Box<[UnsafeCell<MaybeUninit<T>>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> SPMCBroadcasterUnsafeLocalTailsOutsideArc<T, RING_SIZE> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            head: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect::<Vec<_>>().into_boxed_slice(),
        })
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for Arc<SPMCBroadcasterUnsafeLocalTailsOutsideArc<T, RING_SIZE>> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        let head = self.head.load(Ordering::Relaxed);
        let slot = self.ring[head & (RING_SIZE - 1)].get();

        if head >= RING_SIZE && std::mem::needs_drop::<T>() {
            unsafe { (*slot).assume_init_drop(); }
        }

        unsafe { slot.write(MaybeUninit::new(item)); }
        self.head.store(head.wrapping_add(1), Ordering::Release);
    }
}


impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for Arc<SPMCBroadcasterUnsafeLocalTailsOutsideArc<T, RING_SIZE>> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let head = self.head.load(Ordering::Acquire);
            if *local_tail >= head {
                std::hint::spin_loop();
                continue;
            }

            let slot = self.ring[*local_tail & (RING_SIZE - 1)].get();
            let item = unsafe { (*slot).assume_init_ref().clone() };
            *local_tail = local_tail.wrapping_add(1);
            return item;
        }
    }
}
