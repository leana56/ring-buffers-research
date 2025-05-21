use super::PAYLOAD_STASH_GEN_ON_FLY;
use super::PAYLOAD_BYTE_TYPE;
use super::SAMPLE_SIZE;
use super::DistributionType;
use super::RingName;

use serde::Serialize;

// =-= Stash =-= //
// - The stashes are used to pre-populate the payloads for the producers, the idea here was to use a mix of payloads with random bytes and one with blank bytes,
//   as I'm curious to see what the compiler does with the blank payloads and if they simply get optimized out or if they are actually used

// =-= Payloads =-= //
// - The payloads are the actual data that will be sent through the ring, split between one that implements Clone and one that implements Copy
// - This is important because if a ring has multiple consumers and the payload doesn't implement Copy, then the payload will be moved out of the ring and produce UB
// - On the other hand, if the payload implements Clone and the ring is SPSC / MPSC, then the payload will be moved and all will be well


// =----------------------------------------------= //

pub trait AnyPayload: Default + Send + Clone + 'static {
    const COPYABLE: bool;
    const PAYLOAD_SIZE: usize;

    fn update_timestamp(&mut self);
    fn collapse_timestamp(&self) -> (usize, u128);
    fn new_blank(idx: usize) -> Self;
    fn new_random(idx: usize) -> Self;
}

// =----------------------------------------------= //

pub struct PayloadStash<P: AnyPayload> {
    pub payloads: PayloadIterator<P>,
}

impl<P: AnyPayload> PayloadStash<P> {
    pub fn new(ring_name: RingName, consumer_threads: usize, offset: usize) -> Self {
        let send_x_terminations = match ring_name.get_distribution_type() {
            DistributionType::LoadBalancer => consumer_threads,
            DistributionType::Broadcast => 1,
        };

        match PAYLOAD_STASH_GEN_ON_FLY {
            true => PayloadStash {
                payloads: PayloadIterator::OnFly {
                    next_idx: offset,
                    end_idx: offset + SAMPLE_SIZE,
                    send_x_terminations,
                    sent_termination: 0,
                },
            },
            false => PayloadStash {
                payloads: PayloadIterator::PreGenerated {
                    vec: ((0..SAMPLE_SIZE).map(|idx| match PAYLOAD_BYTE_TYPE {
                        PayloadByteType::Random => P::new_random(offset + idx),
                        PayloadByteType::Blank => P::new_blank(offset + idx),
                    }).collect::<Vec<_>>().into_iter().rev().collect()),
                    send_x_terminations,
                    sent_termination: 0,
                },
            }
        }
    }
}

pub enum PayloadIterator<P: AnyPayload> {
    PreGenerated { vec: Vec<P>, send_x_terminations: usize, sent_termination: usize },
    OnFly { next_idx: usize, end_idx: usize, send_x_terminations: usize, sent_termination: usize },
}

impl<P: AnyPayload> Iterator for PayloadIterator<P> {
    type Item = P;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PayloadIterator::PreGenerated { vec, send_x_terminations, sent_termination } => {
                if let Some(payload) = vec.pop() {
                    Some(payload)
                } else if *sent_termination < *send_x_terminations {
                    *sent_termination += 1;
                    Some(P::new_blank(usize::MAX))
                } else {
                    None
                }
            },
            PayloadIterator::OnFly { next_idx, end_idx, send_x_terminations, sent_termination } => {
                if *next_idx < *end_idx {
                    let idx = *next_idx;
                    *next_idx += 1;

                    match PAYLOAD_BYTE_TYPE {
                        PayloadByteType::Random => Some(P::new_random(idx)),
                        PayloadByteType::Blank => Some(P::new_blank(idx)),
                    }
                } else if *sent_termination < *send_x_terminations {
                    *sent_termination += 1;
                    Some(P::new_blank(usize::MAX))
                } else {
                    None
                }
            }
        }
    }
}

// =----------------------------------------------= //

#[derive(Debug, Clone, Serialize)]
pub enum PayloadByteType {
    Random,
    Blank,
}

// =----------------------------------------------= //

#[derive(Debug, Clone, Copy)]
pub struct CopyablePayload<const PAYLOAD_SIZE: usize> {
    pub data: [u8; PAYLOAD_SIZE],
    pub timestamp: minstant::Instant,
    pub idx: usize,
}

impl<const PAYLOAD_SIZE: usize> AnyPayload for CopyablePayload<PAYLOAD_SIZE> {
    const COPYABLE: bool = true;
    const PAYLOAD_SIZE: usize = PAYLOAD_SIZE;

    #[inline]
    fn collapse_timestamp(&self) -> (usize, u128) {
        (self.idx, self.timestamp.elapsed().as_nanos())
    }

    #[inline]
    fn update_timestamp(&mut self) {
        self.timestamp = minstant::Instant::now();
    }

    #[inline]
    fn new_blank(idx: usize) -> Self {
        Self { data: [0; PAYLOAD_SIZE], timestamp: minstant::Instant::now(), idx }
    }

    #[inline]
    fn new_random(idx: usize) -> Self {
        Self { data: rand::random::<[u8; PAYLOAD_SIZE]>(), timestamp: minstant::Instant::now(), idx }
    }
}

impl<const PAYLOAD_SIZE: usize> Default for CopyablePayload<PAYLOAD_SIZE> {
    fn default() -> Self {
        Self::new_blank(0)
    }
}

// =----------------------------------------------= //

#[derive(Debug, Clone)]
pub struct HeapPayload<const PAYLOAD_SIZE: usize> {
    pub data: Box<[u8; PAYLOAD_SIZE]>,
    pub timestamp: minstant::Instant,
    pub idx: usize,
}

impl<const PAYLOAD_SIZE: usize> AnyPayload for HeapPayload<PAYLOAD_SIZE> {
    const COPYABLE: bool = false;
    const PAYLOAD_SIZE: usize = PAYLOAD_SIZE;

    #[inline]
    fn collapse_timestamp(&self) -> (usize, u128) {
        (self.idx, self.timestamp.elapsed().as_nanos())
    }

    #[inline]
    fn update_timestamp(&mut self) {
        self.timestamp = minstant::Instant::now();
    }

    #[inline]
    fn new_blank(idx: usize) -> Self {
        Self { data: Box::new([0; PAYLOAD_SIZE]), timestamp: minstant::Instant::now(), idx }
    }

    #[inline]
    fn new_random(idx: usize) -> Self {
        Self { data: Box::new(rand::random::<[u8; PAYLOAD_SIZE]>()), timestamp: minstant::Instant::now(), idx }
    }
}

impl<const PAYLOAD_SIZE: usize> Default for HeapPayload<PAYLOAD_SIZE> {
    fn default() -> Self {
        Self::new_blank(0)
    }
}