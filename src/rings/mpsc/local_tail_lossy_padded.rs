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
// - Same as "local_tail_lossy" however it uses a padded cache line for the head and sequence counters
// =--------------------------------------------------------= //

unsafe impl<T: AnyPayload, const RING_SIZE: usize> Send for MPSCLocalTailLossyPadded<T, RING_SIZE> {}
unsafe impl<T: AnyPayload, const RING_SIZE: usize> Sync for MPSCLocalTailLossyPadded<T, RING_SIZE> {}

// =--------------------------------------------------------= //

#[repr(align(64))]
pub struct Slot<T: AnyPayload> {
    pub seq: AtomicUsize,
    pub _pad: [u8; 64 - size_of::<AtomicUsize>()],
    pub data: UnsafeCell<MaybeUninit<T>>,
}

impl<T: AnyPayload> Slot<T> {
    pub fn new(index: usize) -> Self {
        Self { seq: AtomicUsize::new(index), _pad: [0; 64 - size_of::<AtomicUsize>()], data: UnsafeCell::new(MaybeUninit::uninit()) }
    }
}

// =--------------------------------------------------------= //

#[repr(align(64))]
pub struct MPSCLocalTailLossyPadded<T: AnyPayload, const RING_SIZE: usize> {
    pub head: AtomicUsize,
    pub _pad: [u8; 64 - size_of::<AtomicUsize>()],
    pub ring: Box<[Slot<T>]>,
}

impl<T: AnyPayload, const RING_SIZE: usize> MPSCLocalTailLossyPadded<T, RING_SIZE> {
    pub fn new() -> (MPSCLocalTailLossyPaddedWriter<T, RING_SIZE>, MPSCLocalTailLossyPaddedReader<T, RING_SIZE>) {
        let inner = Arc::new(Self {
            head: AtomicUsize::new(0),
            _pad: [0; 64 - size_of::<AtomicUsize>()],
            ring: (0..RING_SIZE).map(|i| Slot::new(i)).collect::<Vec<_>>().into_boxed_slice(),
        });

        (MPSCLocalTailLossyPaddedWriter { inner: Arc::clone(&inner) }, MPSCLocalTailLossyPaddedReader { inner })
    }
}


// =-= Writer =-= //

pub struct MPSCLocalTailLossyPaddedWriter<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPSCLocalTailLossyPadded<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPSCLocalTailLossyPaddedWriter<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Sender<T> for MPSCLocalTailLossyPaddedWriter<T, RING_SIZE> {
    #[inline]
    fn push(&self, producer_id: usize, item: T) {

        let timer = minstant::Instant::now();
        loop {
            let head = self.inner.head.load(Ordering::Acquire);
            let slot = &self.inner.ring[head & (RING_SIZE - 1)];
            let seq = slot.seq.load(Ordering::Acquire);
            
            if head == seq {
                match self.inner.head.compare_exchange_weak(head, head.wrapping_add(1), Ordering::AcqRel, Ordering::Relaxed) {
                    Ok(_) => {
                        unsafe { slot.data.get().write(MaybeUninit::new(item)); }
                        slot.seq.store(head.wrapping_add(1), Ordering::Release);
                        return;
                    },
                    Err(_) => continue,
                }
            }

            if timer.elapsed().as_millis() > 1000 {
                return;
            }
            
            std::hint::spin_loop();
        }
    }
}

// =-= Reader =-= //

pub struct MPSCLocalTailLossyPaddedReader<T: AnyPayload, const RING_SIZE: usize> {
    pub inner: Arc<MPSCLocalTailLossyPadded<T, RING_SIZE>>,
}

impl<T: AnyPayload, const RING_SIZE: usize> Clone for MPSCLocalTailLossyPaddedReader<T, RING_SIZE> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<T: AnyPayload, const RING_SIZE: usize> Receiver<T> for MPSCLocalTailLossyPaddedReader<T, RING_SIZE> {
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
            *local_tail = head.saturating_sub(1); // drops any data to catch up with the header
        }
    }
}