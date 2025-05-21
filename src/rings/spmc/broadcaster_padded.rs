use super::{Sender, Receiver};
use crate::AnyPayload;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::mem::size_of;
use std::sync::RwLock;
use std::sync::Weak;
use std::sync::Arc;

// =--------------------------------------------------------= //
// =-= Notes =-= //
// - Unlike the original `broadcaster` implementation, this one uses padding to try and reduce false sharing across the producers & consumers
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for SPMCBroadcasterPadded<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for SPMCBroadcasterPadded<T, RING_SIZE> {}

// =--------------------------------------------------------= //

#[repr(align(64))]
pub struct PaddedAtomicUsize {
    pub value: AtomicUsize,
    _pad: [u8; 64 - size_of::<AtomicUsize>()],
}

impl PaddedAtomicUsize {
    pub fn new(value: usize) -> Self {
        Self { value: AtomicUsize::new(value), _pad: [0; 64 - size_of::<AtomicUsize>()] }
    }
}

// =--------------------------------------------------------= //

#[repr(align(64))]
pub struct SPMCBroadcasterPadded<T: AnyPayload, const RING_SIZE: usize> {
    pub head: PaddedAtomicUsize,
    pub tails: RwLock<Vec<Arc<PaddedAtomicUsize>>>,
    pub ring: Box<[UnsafeCell<MaybeUninit<T>>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> SPMCBroadcasterPadded<T, RING_SIZE> {
    pub fn new() -> (SPMCBroadcasterPaddedWriter<T, RING_SIZE>, SPMCBroadcasterPaddedReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: PaddedAtomicUsize::new(0),
            tails: RwLock::new(Vec::new()),
            ring: (0..RING_SIZE).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect::<Vec<_>>().into_boxed_slice(),
        });

        (SPMCBroadcasterPaddedWriter { inner: Arc::clone(&inner) }, SPMCBroadcasterPaddedReader { tail: Arc::new(PaddedAtomicUsize::new(0)), inner: Arc::clone(&inner) })
    }
}


// =-= Writer =-= //

pub struct SPMCBroadcasterPaddedWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPMCBroadcasterPadded<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPMCBroadcasterPaddedWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for SPMCBroadcasterPaddedWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        let head = self.inner.head.value.load(Ordering::Relaxed);
        let slot = self.inner.ring[head & (RING_SIZE - 1)].get();

        loop {
            let lowest_tail_seq = {
                let tails_lock = self.inner.tails.read().unwrap();
                tails_lock.iter().map(|t| t.value.load(Ordering::Acquire)).min().unwrap_or(head)
            };

            if head.wrapping_sub(lowest_tail_seq) >= RING_SIZE {
                std::hint::spin_loop();
                continue;
            }

            if head >= RING_SIZE && std::mem::needs_drop::<T>() {
                unsafe {
                    (*slot).assume_init_drop();
                }
            }

            unsafe { slot.write(MaybeUninit::new(item)); }
            self.inner.head.value.store(head.wrapping_add(1), Ordering::Release);
            return;
        }
    }
}

// =-= Reader =-= //

pub struct SPMCBroadcasterPaddedReader<T: AnyPayload, const RING_SIZE: usize> {
    pub tail: Arc<PaddedAtomicUsize>,
    pub inner: Arc<SPMCBroadcasterPadded<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPMCBroadcasterPaddedReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        let current_head_seq = self.inner.head.value.load(Ordering::Acquire);

        let new_tail = Arc::new(PaddedAtomicUsize::new(current_head_seq));
        self.inner.tails.write().unwrap().push(Arc::clone(&new_tail));

        Self { tail: new_tail, inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for SPMCBroadcasterPaddedReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let tail = self.tail.value.load(Ordering::Acquire);
            let head = self.inner.head.value.load(Ordering::Acquire);

            if tail >= head {
                std::hint::spin_loop();
                continue;
            }

            let slot = self.inner.ring[tail & (RING_SIZE - 1)].get();
            let item = unsafe { (*slot).assume_init_ref().clone() };

            self.tail.value.store(tail.wrapping_add(1), Ordering::Release);
            return item;
        }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Drop for SPMCBroadcasterPaddedReader<T, RING_SIZE> {
    fn drop(&mut self) {
        let mut tails = self.inner.tails.write().unwrap();
        if let Some(pos) = tails.iter().position(|p| Arc::ptr_eq(p, &self.tail)) {
            tails.swap_remove(pos);
        }
    }
}