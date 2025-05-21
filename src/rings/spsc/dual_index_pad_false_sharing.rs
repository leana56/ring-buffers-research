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
// - Variant of the dual index ring with padding to alleviate any false sharing
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for SPSCDualIndexPadFalseSharing<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for SPSCDualIndexPadFalseSharing<T, RING_SIZE> {}

// =--------------------------------------------------------= //

#[repr(align(64))]
pub struct SPSCDualIndexPadFalseSharing<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    _pad1: [u8; 64 - size_of::<AtomicUsize>()],
    pub tail: AtomicUsize,
    _pad2: [u8; 64 - size_of::<AtomicUsize>()],
    pub ring: Box<[UnsafeCell<MaybeUninit<T>>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> SPSCDualIndexPadFalseSharing<T, RING_SIZE> {
    pub fn new() -> (SPSCDualIndexPadFalseSharingWriter<T, RING_SIZE>, SPSCDualIndexPadFalseSharingReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            _pad1: [0; 64 - size_of::<AtomicUsize>()],
            tail: AtomicUsize::new(0),
            _pad2: [0; 64 - size_of::<AtomicUsize>()],
            ring: (0..RING_SIZE).map(|_| UnsafeCell::new(MaybeUninit::uninit())).collect::<Vec<_>>().into_boxed_slice(),
        });

        (SPSCDualIndexPadFalseSharingWriter { inner: Arc::clone(&inner) }, SPSCDualIndexPadFalseSharingReader { inner })
    }
}

// =-= Writer =-= //

pub struct SPSCDualIndexPadFalseSharingWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPSCDualIndexPadFalseSharing<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPSCDualIndexPadFalseSharingWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for SPSCDualIndexPadFalseSharingWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        loop {
            let head = self.inner.head.load(Ordering::Relaxed);
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

pub struct SPSCDualIndexPadFalseSharingReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPSCDualIndexPadFalseSharing<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPSCDualIndexPadFalseSharingReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for SPSCDualIndexPadFalseSharingReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
            let tail = self.inner.tail.load(Ordering::Relaxed);
            
            if tail >= head {
                std::hint::spin_loop();
                continue;
            }

            let item = unsafe { self.inner.ring[tail & (RING_SIZE - 1)].get().read().assume_init() };
            self.inner.tail.store(tail.wrapping_add(1), Ordering::Release);
            return item;
        }
    }
}
