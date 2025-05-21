use super::{Sender, Receiver};
use crate::AnyPayload;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::RwLock;
use std::sync::Arc;

// =--------------------------------------------------------= //
// =-= Notes =-= //
// - An unsafe variant of the broadcaster ring where instead of the producer keeping track of the tails, each consumer keeps track of their own
// - This has one major benefit and one major drawback:
//    - Benefit: producer can keep populating and doesn't need to wait for the slowest consumer to catch up
//    - Drawback: *Lots of UB* can happen if / when writers lap any of the readers
//        - The UB surface area could be reduced if T implements Copy (write-while-read or read-while-write still possible)
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for SPMCBroadcasterUnsafeLocalTails<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for SPMCBroadcasterUnsafeLocalTails<T, RING_SIZE> {}

// =--------------------------------------------------------= //

pub struct SPMCBroadcasterUnsafeLocalTails<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub ring: Box<[UnsafeCell<MaybeUninit<T>>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> SPMCBroadcasterUnsafeLocalTails<T, RING_SIZE> {
    pub fn new() -> (SPMCBroadcasterUnsafeLocalTailsWriter<T, RING_SIZE>, SPMCBroadcasterUnsafeLocalTailsReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            ring: (0..RING_SIZE).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect::<Vec<_>>().into_boxed_slice(),
        });

        (SPMCBroadcasterUnsafeLocalTailsWriter { inner: Arc::clone(&inner) }, SPMCBroadcasterUnsafeLocalTailsReader { inner })
    }
}


// =-= Writer =-= //

pub struct SPMCBroadcasterUnsafeLocalTailsWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPMCBroadcasterUnsafeLocalTails<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPMCBroadcasterUnsafeLocalTailsWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for SPMCBroadcasterUnsafeLocalTailsWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        let head = self.inner.head.load(Ordering::Relaxed);
        let slot = self.inner.ring[head & (RING_SIZE - 1)].get();

        if head >= RING_SIZE && std::mem::needs_drop::<T>() {
            unsafe { (*slot).assume_init_drop(); }
        }

        unsafe { slot.write(MaybeUninit::new(item)); }
        self.inner.head.store(head.wrapping_add(1), Ordering::Release);
    }
}

// =-= Reader =-= //

pub struct SPMCBroadcasterUnsafeLocalTailsReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPMCBroadcasterUnsafeLocalTails<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPMCBroadcasterUnsafeLocalTailsReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for SPMCBroadcasterUnsafeLocalTailsReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
            if *local_tail >= head {
                std::hint::spin_loop();
                continue;
            }

            let slot = self.inner.ring[*local_tail & (RING_SIZE - 1)].get();
            let item = unsafe { (*slot).assume_init_ref().clone() };
            *local_tail = local_tail.wrapping_add(1);
            return item;
        }
    }
}