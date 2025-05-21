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
// - Same as the safe skipping ring, however instead of wrapping the data in a Box, we'll save the array in the ring itself,
//   the idea being to reduce a pointer indirection from the Box
// - With this method tho we run into the issue of creating a large array on the stack before allocating it to the heap, which could block up the stack and cause an overflow
//   - To alleviate this bug we'll use the fancy Arc::new_uninit() method to allocate the Arc and then directly write the array into the Arc
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for SPSCSafeSkippingNoBoxPtr<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for SPSCSafeSkippingNoBoxPtr<T, RING_SIZE> {}

// =--------------------------------------------------------= //

#[repr(align(64))]
pub struct Slot<T: AnyPayload, const RING_SIZE: usize> {
    pub seq: AtomicUsize,
    _pad: [u8; 64 - size_of::<AtomicUsize>()],
    pub data: UnsafeCell<MaybeUninit<T>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Slot<T, RING_SIZE> {
    pub fn uninit() -> Self {
        Self { 
            seq: AtomicUsize::new(0), 
            _pad: [0; 64 - size_of::<AtomicUsize>()], 
            data: UnsafeCell::new(MaybeUninit::uninit()) 
        }
    }
}

// =--------------------------------------------------------= //


#[repr(align(64))]
pub struct SPSCSafeSkippingNoBoxPtr<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    _pad: [u8; 64 - size_of::<AtomicUsize>()],
    pub ring: [Slot<T, RING_SIZE>; RING_SIZE],
}

impl<T: AnyPayload, const RING_SIZE: usize> SPSCSafeSkippingNoBoxPtr<T, RING_SIZE> {
    pub fn new() -> (SPSCSafeSkippingNoBoxPtrWriter<T, RING_SIZE>, SPSCSafeSkippingNoBoxPtrReader<T, RING_SIZE>) {
        let mut arc = Arc::<Self>::new_uninit();
        let raw: *mut Self = unsafe { Arc::get_mut_unchecked(&mut arc).as_mut_ptr() };

        unsafe {
            (*raw).head = AtomicUsize::new(0);
            (*raw)._pad = [0; 64 - size_of::<AtomicUsize>()];
        }

        for (i, slot) in unsafe { &mut (*raw).ring }.iter_mut().enumerate() {
            unsafe { std::ptr::write(slot, Slot::<T, RING_SIZE>::uninit()); }

            (*slot).seq.store(i, Ordering::Relaxed);
            unsafe { *(*slot).data.get() = MaybeUninit::uninit(); }
        }

        let inner = unsafe { Arc::<MaybeUninit<Self>>::assume_init(arc) };
        (SPSCSafeSkippingNoBoxPtrWriter { inner: Arc::clone(&inner) }, SPSCSafeSkippingNoBoxPtrReader { inner })
    }
}

// =-= Writer =-= //

pub struct SPSCSafeSkippingNoBoxPtrWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPSCSafeSkippingNoBoxPtr<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPSCSafeSkippingNoBoxPtrWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for SPSCSafeSkippingNoBoxPtrWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {
        loop {
            let head = self.inner.head.load(Ordering::Relaxed);
            let slot = &self.inner.ring[head & (RING_SIZE - 1)];
            let seq  = slot.seq.load(Ordering::Acquire);

            if seq != head && seq + RING_SIZE > head {
                std::hint::spin_loop();
                continue;
            }
            
            unsafe { slot.data.get().write(MaybeUninit::new(item)); }
            slot.seq.store(head.wrapping_add(1), Ordering::Release);
            self.inner.head.store(head.wrapping_add(1), Ordering::Relaxed);
            return;
        }
    }
}

// =-= Reader =-= //

pub struct SPSCSafeSkippingNoBoxPtrReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<SPSCSafeSkippingNoBoxPtr<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for SPSCSafeSkippingNoBoxPtrReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for SPSCSafeSkippingNoBoxPtrReader<T, RING_SIZE> {
    #[inline]
    fn pop(&self, local_tail: &mut usize) -> T {
        loop {
            let slot = &self.inner.ring[*local_tail & (RING_SIZE - 1)];
            let seq = slot.seq.load(Ordering::Acquire);
            let expected = local_tail.wrapping_add(1);

            if seq == expected {
                let item = unsafe { slot.data.get().read().assume_init() };
                slot.seq.store(local_tail.wrapping_add(RING_SIZE), Ordering::Release);
                *local_tail = local_tail.wrapping_add(1);
                return item;
            }

            if seq == *local_tail {
                continue;
            }

            let head = self.inner.head.load(Ordering::Acquire);
            *local_tail = head.saturating_sub(1);
        }
    }
}
