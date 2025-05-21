use super::{Sender, Receiver};
use crate::AnyPayload;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Mutex;
use std::sync::Weak;
use std::sync::Arc;

// =--------------------------------------------------------= //
// =-= Notes =-= //
// - Baseline broadcast implementation 
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for SPMCBroadcaster<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for SPMCBroadcaster<T, RING_SIZE> {}

// =--------------------------------------------------------= //

pub struct SPMCBroadcaster<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub tails: Mutex<Vec<Arc<AtomicUsize>>>,
    pub ring: Box<[UnsafeCell<MaybeUninit<T>>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> SPMCBroadcaster<T, RING_SIZE> {
    pub fn new() -> (SPMCBroadcasterWriter<T, RING_SIZE>, SPMCBroadcasterReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            tails: Mutex::new(Vec::new()),
            ring: (0..RING_SIZE).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect::<Vec<_>>().into_boxed_slice(),
        });

        (SPMCBroadcasterWriter { inner: Arc::clone(&inner) }, SPMCBroadcasterReader { tail: Arc::new(AtomicUsize::new(0)), inner: Arc::clone(&inner) })
    }
}


// =-= Writer =-= //

pub struct SPMCBroadcasterWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPMCBroadcaster<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPMCBroadcasterWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for SPMCBroadcasterWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        loop {
            let head = self.inner.head.load(Ordering::Acquire);

            let lowest_tail_seq = {
                let tails_lock = self.inner.tails.lock().unwrap();
                tails_lock.iter().map(|t| t.load(Ordering::Acquire)).min().unwrap_or(head)
            };

            if head.wrapping_sub(lowest_tail_seq) >= RING_SIZE {
                std::hint::spin_loop();
                continue;
            }

            let slot = self.inner.ring[head & (RING_SIZE - 1)].get();
            if head > RING_SIZE && std::mem::needs_drop::<T>() {
                unsafe { (*slot).assume_init_drop(); }
            }

            unsafe { slot.write(MaybeUninit::new(item)); }
            self.inner.head.store(head.wrapping_add(1), Ordering::Release);
            return;
        }
    }
}

// =-= Reader =-= //

pub struct SPMCBroadcasterReader<T: AnyPayload, const RING_SIZE: usize> {
    pub tail: Arc<AtomicUsize>,
    pub inner: Arc<SPMCBroadcaster<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPMCBroadcasterReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        let current_head_seq = self.inner.head.load(Ordering::Acquire);

        let new_tail = Arc::new(AtomicUsize::new(current_head_seq));
        self.inner.tails.lock().unwrap().push(Arc::clone(&new_tail));

        Self { tail: new_tail, inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for SPMCBroadcasterReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
            let consumers_tail = self.tail.load(Ordering::Acquire);

            if consumers_tail >= head {
                std::hint::spin_loop();
                continue;
            }

            let slot = self.inner.ring[consumers_tail & (RING_SIZE - 1)].get();
            let item = unsafe { (*slot).assume_init_ref().clone() };
            self.tail.store(consumers_tail.wrapping_add(1), Ordering::Release);
            return item;
        }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Drop for SPMCBroadcasterReader<T, RING_SIZE> {
    fn drop(&mut self) {
        let mut tails = self.inner.tails.lock().unwrap();
        if let Some(pos) = tails.iter().position(|p| Arc::ptr_eq(p, &self.tail)) {
            tails.swap_remove(pos);
        }
    }
}