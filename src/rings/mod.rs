use crate::AnyPayload;

pub mod spsc;
pub use spsc::*;

pub mod spmc;
pub use spmc::*;

pub mod mpsc;
pub use mpsc::*;

pub mod mpmc;
pub use mpmc::*;

use serde::Serialize;
use std::fmt;

// =----------------------------------------------= //
// - All functions are busy-spinning:
//   for implementations that use backpressure on pushes, they'll spin till available
//   for implementations that are awaiting new data on pop, they'll spin till fresh data is available

pub trait Sender<P: AnyPayload> {
    fn push(&self, producer_id: usize, item: P);
}

pub trait Receiver<P: AnyPayload> {
    fn pop(&self, local_tail: &mut usize) -> P;
}

// =----------------------------------------------= //

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RingName {
    // =-= MPMC =-= //
    MPMCLoadBalancer,
    MPMCLoadBalancerPadded,
    MPMCBroadcaster,
    MPMCBroadcasterUnsafeIndivSPMCCopy,
    // .. add more MPMC variants here
    
    // =-= MPSC =-= //
    MPSCLocalTailLossy,
    MPSCLocalTailLossyPadded,
    MPSCGlobalTail,
    MPSCGlobalTailLossy,
    MPSCIndivSPSCGroup,
    // .. add more MPSC variants here

    // =-= SPMC =-= //
    SPMCLoadBalancerCopy,
    SPMCBroadcaster,
    SPMCBroadcasterPadded,
    SPMCBroadcasterUnsafeLocalTails,
    SPMCBroadcasterUnsafeLocalTailsOutsideArc,
    // .. add more SPMC variants here

    // =-= SPSC =-= //
    SPSCDualIndexFalseSharing,
    SPSCDualIndexPadFalseSharing,
    SPSCSafeSkipping,
    SPSCSafeSkippingNoBoxPtr,
    SPSCSafeSkippingOutsideArc,
    SPSCSlotLockLocalTailCopy,
    SPSCFullLockLocalTailCopy,
    // .. add more SPSC variants here
}

impl fmt::Display for RingName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)      // or a custom string if you prefer
    }
}

impl RingName {
    pub const fn get_channel_type(&self) -> ChannelType {
        match self {
            // =-= MPMC =-= //
            RingName::MPMCLoadBalancer => ChannelType::MPMC,
            RingName::MPMCLoadBalancerPadded => ChannelType::MPMC,
            RingName::MPMCBroadcaster => ChannelType::MPMC,
            RingName::MPMCBroadcasterUnsafeIndivSPMCCopy => ChannelType::MPMC,
            // .. add more MPMC variants here

            // =-= MPSC =-= //
            RingName::MPSCLocalTailLossy => ChannelType::MPSC,
            RingName::MPSCLocalTailLossyPadded => ChannelType::MPSC,
            RingName::MPSCGlobalTail => ChannelType::MPSC,
            RingName::MPSCGlobalTailLossy => ChannelType::MPSC,
            RingName::MPSCIndivSPSCGroup => ChannelType::MPSC,
            // .. add more MPSC variants here

            // =-= SPMC =-= //
            RingName::SPMCLoadBalancerCopy => ChannelType::SPMC,
            RingName::SPMCBroadcaster => ChannelType::SPMC,
            RingName::SPMCBroadcasterPadded => ChannelType::SPMC,
            RingName::SPMCBroadcasterUnsafeLocalTails => ChannelType::SPMC,
            RingName::SPMCBroadcasterUnsafeLocalTailsOutsideArc => ChannelType::SPMC,
            // .. add more SPMC variants here

            // =-= SPSC =-= //
            RingName::SPSCDualIndexFalseSharing => ChannelType::SPSC,
            RingName::SPSCDualIndexPadFalseSharing => ChannelType::SPSC,
            RingName::SPSCSafeSkipping => ChannelType::SPSC,
            RingName::SPSCSafeSkippingNoBoxPtr => ChannelType::SPSC,
            RingName::SPSCSafeSkippingOutsideArc => ChannelType::SPSC,
            RingName::SPSCSlotLockLocalTailCopy => ChannelType::SPSC,
            RingName::SPSCFullLockLocalTailCopy => ChannelType::SPSC,
            // .. add more SPSC variants here

        }
    }
    pub const fn get_distribution_type(&self) -> DistributionType {
        match self {
            // =-= MPMC =-= //
            RingName::MPMCLoadBalancer => DistributionType::LoadBalancer,
            RingName::MPMCLoadBalancerPadded => DistributionType::LoadBalancer,
            RingName::MPMCBroadcaster => DistributionType::Broadcast,
            RingName::MPMCBroadcasterUnsafeIndivSPMCCopy => DistributionType::Broadcast,
            // .. add more MPMC variants here
            
            // =-= MPSC =-= //
            RingName::MPSCLocalTailLossy => DistributionType::Broadcast,
            RingName::MPSCLocalTailLossyPadded => DistributionType::Broadcast,
            RingName::MPSCGlobalTail => DistributionType::Broadcast,
            RingName::MPSCGlobalTailLossy => DistributionType::Broadcast,
            RingName::MPSCIndivSPSCGroup => DistributionType::Broadcast,
            // .. add more MPSC variants here

            // =-= SPMC =-= //
            RingName::SPMCLoadBalancerCopy => DistributionType::LoadBalancer,
            RingName::SPMCBroadcaster => DistributionType::Broadcast,
            RingName::SPMCBroadcasterPadded => DistributionType::Broadcast,
            RingName::SPMCBroadcasterUnsafeLocalTails => DistributionType::Broadcast,
            RingName::SPMCBroadcasterUnsafeLocalTailsOutsideArc => DistributionType::Broadcast,
            // .. add more SPMC variants here

            // =-= SPSC =-= //
            // - Broadcasts to single consumer lolol
            RingName::SPSCDualIndexFalseSharing => DistributionType::Broadcast,
            RingName::SPSCDualIndexPadFalseSharing => DistributionType::Broadcast,
            RingName::SPSCSafeSkipping => DistributionType::Broadcast,
            RingName::SPSCSafeSkippingNoBoxPtr => DistributionType::Broadcast,
            RingName::SPSCSafeSkippingOutsideArc => DistributionType::Broadcast,
            RingName::SPSCSlotLockLocalTailCopy => DistributionType::Broadcast,
            RingName::SPSCFullLockLocalTailCopy => DistributionType::Broadcast,
            // .. add more SPSC variants here
        }
    }
}

#[derive(Debug)]
pub enum ChannelType {
    SPSC,
    SPMC,
    MPSC,
    MPMC,
}

#[derive(Debug)]
pub enum DistributionType {
    LoadBalancer,
    Broadcast,
}
