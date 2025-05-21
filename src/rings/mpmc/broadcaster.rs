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
// - Simple (no optimizations) broadcaster / fan-out implementation using the classic seq / disruptor pattern as seen in other implementations
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for MPMCBroadcaster<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for MPMCBroadcaster<T, RING_SIZE> {}

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

pub struct MPMCBroadcaster<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub tails: Mutex<Vec<Arc<AtomicUsize>>>,
    pub ring: Box<[Slot<T>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> MPMCBroadcaster<T, RING_SIZE> {
    pub fn new() -> (MPMCBroadcasterWriter<T, RING_SIZE>, MPMCBroadcasterReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            tails: Mutex::new(Vec::new()),
            ring: (0..RING_SIZE).map(|i| Slot::new(i)).collect::<Vec<_>>().into_boxed_slice(),
        });

        (MPMCBroadcasterWriter { inner: Arc::clone(&inner) }, MPMCBroadcasterReader { tail: Arc::new(AtomicUsize::new(0)), inner: Arc::clone(&inner) })
    }
}

// =-= Writer =-= //

pub struct MPMCBroadcasterWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPMCBroadcaster<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPMCBroadcasterWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for MPMCBroadcasterWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {

        let timer = minstant::Instant::now();
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
            let lowest_tail_seq = {
                let tails_lock = self.inner.tails.lock().unwrap();
                tails_lock.iter().map(|t| t.load(Ordering::Acquire)).min().unwrap_or(head)
            };

            if timer.elapsed().as_millis() > 1000 {
                return;
            }

            if head.wrapping_sub(lowest_tail_seq) >= RING_SIZE {
                std::hint::spin_loop();
                continue;
            }

            if self.inner.head.compare_exchange_weak(head, head.wrapping_add(1), Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                let slot = &self.inner.ring[head & (RING_SIZE - 1)];

                if head > RING_SIZE && std::mem::needs_drop::<T>() {
                    unsafe { (*slot.data.get()).assume_init_drop(); }
                }
                
                unsafe { slot.data.get().write(MaybeUninit::new(item)); }
                slot.seq.store(head.wrapping_add(1), Ordering::Release);
                return;
            }

            std::hint::spin_loop();
        }
    }
}

// =-= Reader =-= //

pub struct MPMCBroadcasterReader<T: AnyPayload, const RING_SIZE: usize> {
    pub tail: Arc<AtomicUsize>,
    pub inner: Arc<MPMCBroadcaster<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPMCBroadcasterReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        let current_head_seq = self.inner.head.load(Ordering::Acquire);

        let new_tail = Arc::new(AtomicUsize::new(current_head_seq));
        self.inner.tails.lock().unwrap().push(Arc::clone(&new_tail));

        Self { tail: new_tail, inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for MPMCBroadcasterReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
            if *local_tail >= head {
                std::hint::spin_loop();
                continue;
            }

            let slot = &self.inner.ring[*local_tail & (RING_SIZE - 1)];
            let seq = slot.seq.load(Ordering::Acquire);
            let expected = *local_tail + 1;

            if seq == expected {
                let value = unsafe { (*slot.data.get()).assume_init_ref().clone() };
                let new_tail = *local_tail + 1;
                *local_tail = new_tail;

                self.tail.store(new_tail, Ordering::Release);
                return value;
            }
        }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Drop for MPMCBroadcasterReader<T, RING_SIZE> {
    fn drop(&mut self) {
        let mut tails = self.inner.tails.lock().unwrap();
        if let Some(pos) = tails.iter().position(|p| Arc::ptr_eq(p, &self.tail)) {
            tails.swap_remove(pos);
        }
    }
}