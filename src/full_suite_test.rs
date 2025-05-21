use super::*;

// =-= Full Suite Testing =-= //
// - Macros didn't sound fun at the time so we have a static amount of payload & ring size combinations here
// - To decide which combinations you'd like to run, feel free to change in the main.rs file (FULL_SUITE_PAYLOAD_SIZES & FULL_SUITE_RING_SIZES), tho make sure they're available here (all the ones )

pub fn run_full_suite_testing() {
    for payload_size in FULL_SUITE_PAYLOAD_SIZES {
        for ring_size in FULL_SUITE_RING_SIZES {
            match (payload_size, ring_size) {
                (2, 4) => run_all_full_suite_tests::<CopyablePayload<2>, 2, 4>(),
                (8, 4) => run_all_full_suite_tests::<CopyablePayload<8>, 8, 4>(),
                (16, 4) => run_all_full_suite_tests::<CopyablePayload<16>, 16, 4>(),
                (32, 4) => run_all_full_suite_tests::<CopyablePayload<32>, 32, 4>(),
                (64, 4) => run_all_full_suite_tests::<CopyablePayload<64>, 64, 4>(),
                (128, 4) => run_all_full_suite_tests::<CopyablePayload<128>, 128, 4>(),
                (256, 4) => run_all_full_suite_tests::<CopyablePayload<256>, 256, 4>(),
                (1024, 4) => run_all_full_suite_tests::<CopyablePayload<1024>, 1024, 4>(),
                (2048, 4) => run_all_full_suite_tests::<CopyablePayload<2048>, 2048, 4>(),
                (4096, 4) => run_all_full_suite_tests::<CopyablePayload<4096>, 4096, 4>(),

                (2, 8) => run_all_full_suite_tests::<CopyablePayload<2>, 2, 8>(),
                (8, 8) => run_all_full_suite_tests::<CopyablePayload<8>, 8, 8>(),
                (16, 8) => run_all_full_suite_tests::<CopyablePayload<16>, 16, 8>(),
                (32, 8) => run_all_full_suite_tests::<CopyablePayload<32>, 32, 8>(),
                (64, 8) => run_all_full_suite_tests::<CopyablePayload<64>, 64, 8>(),
                (128, 8) => run_all_full_suite_tests::<CopyablePayload<128>, 128, 8>(),
                (256, 8) => run_all_full_suite_tests::<CopyablePayload<256>, 256, 8>(),
                (1024, 8) => run_all_full_suite_tests::<CopyablePayload<1024>, 1024, 8>(),
                (2048, 8) => run_all_full_suite_tests::<CopyablePayload<2048>, 2048, 8>(),
                (4096, 8) => run_all_full_suite_tests::<CopyablePayload<4096>, 4096, 8>(),

                (2, 16) => run_all_full_suite_tests::<CopyablePayload<2>, 2, 16>(),
                (8, 16) => run_all_full_suite_tests::<CopyablePayload<8>, 8, 16>(),
                (16, 16) => run_all_full_suite_tests::<CopyablePayload<16>, 16, 16>(),
                (32, 16) => run_all_full_suite_tests::<CopyablePayload<32>, 32, 16>(),
                (64, 16) => run_all_full_suite_tests::<CopyablePayload<64>, 64, 16>(),
                (128, 16) => run_all_full_suite_tests::<CopyablePayload<128>, 128, 16>(),
                (256, 16) => run_all_full_suite_tests::<CopyablePayload<256>, 256, 16>(),
                (1024, 16) => run_all_full_suite_tests::<CopyablePayload<1024>, 1024, 16>(),
                (2048, 16) => run_all_full_suite_tests::<CopyablePayload<2048>, 2048, 16>(),
                (4096, 16) => run_all_full_suite_tests::<CopyablePayload<4096>, 4096, 16>(),

                (2, 32) => run_all_full_suite_tests::<CopyablePayload<2>, 2, 32>(),
                (8, 32) => run_all_full_suite_tests::<CopyablePayload<8>, 8, 32>(),
                (16, 32) => run_all_full_suite_tests::<CopyablePayload<16>, 16, 32>(),
                (32, 32) => run_all_full_suite_tests::<CopyablePayload<32>, 32, 32>(),
                (64, 32) => run_all_full_suite_tests::<CopyablePayload<64>, 64, 32>(),
                (128, 32) => run_all_full_suite_tests::<CopyablePayload<128>, 128, 32>(),
                (256, 32) => run_all_full_suite_tests::<CopyablePayload<256>, 256, 32>(),
                (1024, 32) => run_all_full_suite_tests::<CopyablePayload<1024>, 1024, 32>(),
                (2048, 32) => run_all_full_suite_tests::<CopyablePayload<2048>, 2048, 32>(),
                (4096, 32) => run_all_full_suite_tests::<CopyablePayload<4096>, 4096, 32>(),
                
                (2, 64) => run_all_full_suite_tests::<CopyablePayload<2>, 2, 64>(),
                (8, 64) => run_all_full_suite_tests::<CopyablePayload<8>, 8, 64>(),
                (16, 64) => run_all_full_suite_tests::<CopyablePayload<16>, 16, 64>(),
                (32, 64) => run_all_full_suite_tests::<CopyablePayload<32>, 32, 64>(),
                (64, 64) => run_all_full_suite_tests::<CopyablePayload<64>, 64, 64>(),
                (128, 64) => run_all_full_suite_tests::<CopyablePayload<128>, 128, 64>(),
                (256, 64) => run_all_full_suite_tests::<CopyablePayload<256>, 256, 64>(),
                (1024, 64) => run_all_full_suite_tests::<CopyablePayload<1024>, 1024, 64>(),
                (2048, 64) => run_all_full_suite_tests::<CopyablePayload<2048>, 2048, 64>(),
                (4096, 64) => run_all_full_suite_tests::<CopyablePayload<4096>, 4096, 64>(),

                (2, 256) => run_all_full_suite_tests::<CopyablePayload<2>, 2, 256>(),
                (8, 256) => run_all_full_suite_tests::<CopyablePayload<8>, 8, 256>(),
                (16, 256) => run_all_full_suite_tests::<CopyablePayload<16>, 16, 256>(),
                (32, 256) => run_all_full_suite_tests::<CopyablePayload<32>, 32, 256>(),
                (64, 256) => run_all_full_suite_tests::<CopyablePayload<64>, 64, 256>(),
                (128, 256) => run_all_full_suite_tests::<CopyablePayload<128>, 128, 256>(),
                (256, 256) => run_all_full_suite_tests::<CopyablePayload<256>, 256, 256>(),
                (1024, 256) => run_all_full_suite_tests::<CopyablePayload<1024>, 1024, 256>(),
                (2048, 256) => run_all_full_suite_tests::<CopyablePayload<2048>, 2048, 256>(),
                (4096, 256) => run_all_full_suite_tests::<CopyablePayload<4096>, 4096, 256>(),

                (2, 1024) => run_all_full_suite_tests::<CopyablePayload<2>, 2, 1024>(),
                (8, 1024) => run_all_full_suite_tests::<CopyablePayload<8>, 8, 1024>(),
                (16, 1024) => run_all_full_suite_tests::<CopyablePayload<16>, 16, 1024>(),
                (32, 1024) => run_all_full_suite_tests::<CopyablePayload<32>, 32, 1024>(),
                (64, 1024) => run_all_full_suite_tests::<CopyablePayload<64>, 64, 1024>(),
                (128, 1024) => run_all_full_suite_tests::<CopyablePayload<128>, 128, 1024>(),
                (256, 1024) => run_all_full_suite_tests::<CopyablePayload<256>, 256, 1024>(),
                (1024, 1024) => run_all_full_suite_tests::<CopyablePayload<1024>, 1024, 1024>(),
                (2048, 1024) => run_all_full_suite_tests::<CopyablePayload<2048>, 2048, 1024>(),
                (4096, 1024) => run_all_full_suite_tests::<CopyablePayload<4096>, 4096, 1024>(),

                (2, 4096) => run_all_full_suite_tests::<CopyablePayload<2>, 2, 4096>(),
                (8, 4096) => run_all_full_suite_tests::<CopyablePayload<8>, 8, 4096>(),
                (16, 4096) => run_all_full_suite_tests::<CopyablePayload<16>, 16, 4096>(),
                (32, 4096) => run_all_full_suite_tests::<CopyablePayload<32>, 32, 4096>(),
                (64, 4096) => run_all_full_suite_tests::<CopyablePayload<64>, 64, 4096>(),
                (128, 4096) => run_all_full_suite_tests::<CopyablePayload<128>, 128, 4096>(),
                (256, 4096) => run_all_full_suite_tests::<CopyablePayload<256>, 256, 4096>(),
                (1024, 4096) => run_all_full_suite_tests::<CopyablePayload<1024>, 1024, 4096>(),
                (2048, 4096) => run_all_full_suite_tests::<CopyablePayload<2048>, 2048, 4096>(),
                (4096, 4096) => run_all_full_suite_tests::<CopyablePayload<4096>, 4096, 4096>(),
                
                _ => continue,
            }
        }
    }
}

fn run_all_full_suite_tests<T: AnyPayload, const PAYLOAD_SIZE: usize, const RING_SIZE: usize>() {
    // =-= SPSC =-= //
    if RUN_SPSC_EXPERIMENTS {
        println!("{}[*] PAYLOAD_SIZE: {} | RING_SIZE: {} |:| Starting SPSC Ring Buffer Test{}\n", CL::Dull.get(), T::PAYLOAD_SIZE, RING_SIZE, CL::End.get());
        let mut grouped_measurements = GroupMeasurements::new();
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPSCDualIndexFalseSharing, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPSCDualIndexPadFalseSharing, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPSCSafeSkipping, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPSCSafeSkippingNoBoxPtr, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPSCSafeSkippingOutsideArc, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPSCSlotLockLocalTailCopy, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPSCFullLockLocalTailCopy, &mut grouped_measurements);
        // .. add more SPSC variants here
        grouped_measurements.print_relative_results();
    }
    

    // =-= SPMC =-= //
    if RUN_SPMC_EXPERIMENTS {
        println!("\n\n{}[*] PAYLOAD_SIZE: {} | RING_SIZE: {} |:| Starting SPMC Ring Buffer Test{}\n", CL::Dull.get(), T::PAYLOAD_SIZE, RING_SIZE, CL::End.get());
        let mut grouped_measurements = GroupMeasurements::new();
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPMCLoadBalancerCopy, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPMCBroadcaster, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPMCBroadcasterPadded, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPMCBroadcasterUnsafeLocalTails, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::SPMCBroadcasterUnsafeLocalTailsOutsideArc, &mut grouped_measurements);
        // .. add more SPMC variants here
        grouped_measurements.print_relative_results();
    }


    // =-= MPSC =-= //
    if RUN_MPSC_EXPERIMENTS {
        println!("\n\n{}[*] PAYLOAD_SIZE: {} | RING_SIZE: {} |:| Starting MPSC Ring Buffer Test{}\n", CL::Dull.get(), T::PAYLOAD_SIZE, RING_SIZE, CL::End.get());
        let mut grouped_measurements = GroupMeasurements::new();
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::MPSCLocalTailLossy, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::MPSCLocalTailLossyPadded, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::MPSCGlobalTail, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::MPSCGlobalTailLossy, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::MPSCIndivSPSCGroup, &mut grouped_measurements);
        // .. add more MPSC variants here
        grouped_measurements.print_relative_results();
    }
    

    // =-= MPMC =-= //
    if RUN_MPMC_EXPERIMENTS {
        println!("\n\n{}[*] PAYLOAD_SIZE: {} | RING_SIZE: {} |:| Starting MPMC Ring Buffer Test{}\n", CL::Dull.get(), T::PAYLOAD_SIZE, RING_SIZE, CL::End.get());
        let mut grouped_measurements = GroupMeasurements::new();
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::MPMCLoadBalancer, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::MPMCLoadBalancerPadded, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::MPMCBroadcaster, &mut grouped_measurements);
        start_experiment::<T, PAYLOAD_SIZE, RING_SIZE>(RingName::MPMCBroadcasterUnsafeIndivSPMCCopy, &mut grouped_measurements);
        // .. add more MPMC variants here
        grouped_measurements.print_relative_results();
    }
    

    println!("{}[*] PAYLOAD_SIZE: {} | RING_SIZE: {} |:| All experiments complete!{}\n\n", CL::LimeGreen.get(), T::PAYLOAD_SIZE, RING_SIZE, CL::End.get());
}