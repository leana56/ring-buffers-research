#![feature(get_mut_unchecked, new_uninit)]

use std::sync::Arc;

mod rings;
use rings::*;

mod measurements;
use measurements::group::*;
use measurements::individual::*;

mod payload;
use payload::*;

mod experiment;
use experiment::*;

mod full_suite_test;
use full_suite_test::*;

mod utils;
use utils::*;

// =---------------------------------------------------------------------------------------------------------------------------------= //
// =-=-=-= Constants =-=-=-= //

const FULL_SUITE_TESTING: bool = false; // *Takes much longer to compile | If true, will run all the experiments under varying payload & ring size combinations (All else the same; CopyablePayload only) | Doesn't include different thread counts
const FULL_SUITE_PAYLOAD_SIZES: [usize; 11] = [2, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096]; // Available payload sizes | Feel free to remove any you don't want to test
const FULL_SUITE_RING_SIZES: [usize; 8] = [4, 8, 16, 32, 64, 256, 1024, 4096]; // Available ring sizes | Feel free to remove any you don't want to test

// ----------------------------// 

const CONSECUTIVE_RUNS: usize = 1; // How many times to run all the experiments (loop after each complete run) | *Does not affect full suite testing

const SAMPLE_SIZE: usize = 1_000_000; // How many samples to send through each individual producer thread (more the better to reduce variance)

const PAYLOAD_SIZE: usize = 256; // How many bytes to send through each individual producer thread at a time (except full_suite)
const PAYLOAD_BYTE_TYPE: PayloadByteType = PayloadByteType::Random;
const PAYLOAD_STASH_GEN_ON_FLY: bool = false; // false -> pre generates *all* samples (careful, takes up a lot of memory) | true -> generates samples on iter

const BURN_PRODUCER_TIME: u64 = 0; // In micros, how long would you like to wait between sending samples? (0 = no burn, continue busy spinning sending new data)
const BURN_CONSUMER_TIME: u64 = 0; // In micros, how long would you like to burn after receiving a new sample? (0 = no burn, continue busy spinning for new data)

const MEASUREMENT_SAMPLE_PERCENTAGE: usize = 100; // Granularity of latency measurements | Depending on the sample size, a large percentage can be *quite* memory intensive
const SAVE_MEASUREMENTS: bool = true; // Saves results of the measurements to a file | Doesn't overwrite previous results, appends to the file in JSON format

// =-=-=-= Pinning =-=-=-= //
// - Depending on the ring, we will spin up a producer / consumer thread on a specific CPU core,
//   for each additional producer / consumer thread, we'll increment the index of the CPU core (i.e. 1st producer -> CPU Core 1, 2nd producer -> CPU Core 2, etc.)
// - Recommended to set isolcpus / interrupts handlers to a core that is not being used by the ring for cleaner measurements

const PRODUCER_PIN_CPU_IDS: [usize; 5] = [1, 2, 3, 4, 5];
const CONSUMER_PIN_CPU_IDS: [usize; 5] = [6, 7, 8, 9, 10];


// =-=-=-= Parameters by Channel Type =-=-=-= //

// =-= SPSC =-= //
const RUN_SPSC_EXPERIMENTS: bool = true;
const SPSC_RING_SIZE: usize = 1024; // (standard impl's require power of 2) | Default ring size unless otherwise specified in the implementation

// =-= SPMC =-= //
// - Single Producer, Multiple Consumers
const RUN_SPMC_EXPERIMENTS: bool = true;
const SPMC_RING_SIZE: usize = 1024; // (standard impl's require power of 2) | Default ring size unless otherwise specified in the implementation
const SPMC_NUM_CONSUMERS: usize = 2;

// =-= MPSC =-= //
// - Multiple Producers, Single Consumer
const RUN_MPSC_EXPERIMENTS: bool = true;
const MPSC_RING_SIZE: usize = 1024; // (standard impl's require power of 2) | Default ring size unless otherwise specified in the implementation
const MPSC_NUM_PRODUCERS: usize = 2;

// =-= MPMC =-= //
// - Multiple Producers, Multiple Consumers
const RUN_MPMC_EXPERIMENTS: bool = true;
const MPMC_RING_SIZE: usize = 1024; // (standard impl's require power of 2) | Default ring size unless otherwise specified in the implementation
const MPMC_NUM_PRODUCERS: usize = 2;
const MPMC_NUM_CONSUMERS: usize = 2;

// =---------------------------------------------------------------------------------------------------------------------------------= //


fn main() {
    // =-= Assertions =-= //
    assert!(SPMC_NUM_CONSUMERS < CONSUMER_PIN_CPU_IDS.len() + 1, "SPMC_NUM_CONSUMERS must be less than the len CONSUMER_PIN_CPU_IDS");
    assert!(MPSC_NUM_PRODUCERS < PRODUCER_PIN_CPU_IDS.len() + 1, "MPSC_NUM_PRODUCERS must be less than the len PRODUCER_PIN_CPU_IDS");
    assert!(MPMC_NUM_PRODUCERS < PRODUCER_PIN_CPU_IDS.len() + 1, "MPMC_NUM_PRODUCERS must be less than the len PRODUCER_PIN_CPU_IDS");
    assert!(MPMC_NUM_CONSUMERS < CONSUMER_PIN_CPU_IDS.len() + 1, "MPMC_NUM_CONSUMERS must be less than the len CONSUMER_PIN_CPU_IDS");

    let cumulative_payload_size = (SAMPLE_SIZE * std::mem::size_of::<CopyablePayload<PAYLOAD_SIZE>>()) as f64 / 1_000_000_000.0;
    if cumulative_payload_size > 1.0 && (PAYLOAD_STASH_GEN_ON_FLY == false || MEASUREMENT_SAMPLE_PERCENTAGE > 50) {
        println!("{}[!] WARNING: TOTAL CUMULATIVE PAYLOAD SIZE is greater than 1 GB ({:.2} GB) and PAYLOAD_STASH_GEN_ON_FLY is false or MEASUREMENT_SAMPLE_PERCENTAGE is greater than 50 ({}%). This has the potential to use *a lot* of memory and potentially crash your computer. Proceed with caution!{}", CL::Red.get(), cumulative_payload_size, MEASUREMENT_SAMPLE_PERCENTAGE, CL::End.get());
        println!("{}[-] Press Enter to continue...{}", CL::Dull.get(), CL::End.get());
        let _ = std::io::stdin().read_line(&mut String::new());
    }

    // =-= Create results directory =-= //
    std::fs::create_dir_all("results").unwrap();
    std::fs::create_dir_all("results/SPSC").unwrap();
    std::fs::create_dir_all("results/SPMC").unwrap();
    std::fs::create_dir_all("results/MPSC").unwrap();
    std::fs::create_dir_all("results/MPMC").unwrap();


    // ========================================================================================== //
    // =-= Adding new ring implementations =-= //
    // 1. Decide which channel type you're interested in (SPSC is the simplest to start with)
    // 2. Clone one of the implementations (if you'd like to start from a baseline)
    // 3. Create new file under that channel type's folder
    // 4. Change the name of the ring to something unique
    // 5. Implement your targeted changes / tuning
    // 6. Add the ring details to rings/{channel_type}/mod.rs, rings/mod.rs, experiment.rs, full_suite_test.rs, and main.rs (notes are laid out for guidance)
    // 7. Good to go!

    // ========================================================================================== //
    // =-= Notes =-= //
    // - The goal of this repo is to provide a reference implementation of a ring buffer, and a framework for you to easily build new variants of your own, with the benefit of a customizable benchmarking suite in a somewhat live environment (unlike other testing frameworks like criterion)
    // - Thus, these implementations are for *educational purposes* only, although most implementations are safe to use, some variants are designed to be exposed to different types of UB to target unique tradeoffs
    // - I highly recommend checking out https://github.com/crossbeam-rs/crossbeam/blob/master/crossbeam-queue/src/array_queue.rs for a production ready implementation
    // - Enjoy!

    // =-= Latency Analysis =-= //
    // - If your max latency is very high, the bottleneck is likely on the consumer side as the latency includes the time waiting in the queue
    // - If your max latency is low, the bottleneck is likely on the producer side (can happen when the PAYLOAD_SIZE is large and on-the-fly generation is enabled)

    // =-= Measurement Architecture =-= //
    // - In the MPSC and MPMC variants you'll see there is a timer in the push() functions, this is for synchronization between the producers (otherwise hangs when the consumer gets the termination signal from one producer, leaves, and the other producer keeps trying to push their data but due to backpressure mechanisms get stuck in loop)
    // - Let me know what you think and if you come up with any better solutions pls!

    // ========================================================================================== //
    // Convenience type for changing the payload type of all experiments at once, feel free to change any individual experiment's payload type to whatever you'd like
    type BlanketPayloadChange<const PAYLOAD_SIZE: usize> = CopyablePayload<PAYLOAD_SIZE>; // CopyablePayload | HeapPayload

    for run_num in 0..CONSECUTIVE_RUNS {

        // =-= SPSC =-= //
        if RUN_SPSC_EXPERIMENTS {
            println!("{}[*] ({}/{}) | Starting SPSC Ring Buffer Test{}\n", CL::Dull.get(), run_num + 1, CONSECUTIVE_RUNS, CL::End.get());
            let mut grouped_measurements = GroupMeasurements::new();
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPSC_RING_SIZE>(RingName::SPSCDualIndexFalseSharing, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPSC_RING_SIZE>(RingName::SPSCDualIndexPadFalseSharing, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPSC_RING_SIZE>(RingName::SPSCSafeSkipping, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPSC_RING_SIZE>(RingName::SPSCSafeSkippingNoBoxPtr, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPSC_RING_SIZE>(RingName::SPSCSafeSkippingOutsideArc, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPSC_RING_SIZE>(RingName::SPSCSlotLockLocalTailCopy, &mut grouped_measurements); // Enforces Copy trait on payload
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPSC_RING_SIZE>(RingName::SPSCFullLockLocalTailCopy, &mut grouped_measurements); // Enforces Copy trait on payload
            // .. add more SPSC variants here
            grouped_measurements.print_relative_results();
        }


        // =-= SPMC =-= //
        if RUN_SPMC_EXPERIMENTS {
            println!("\n\n{}[*] ({}/{}) | Starting SPMC Ring Buffer Test{}\n", CL::Dull.get(), run_num + 1, CONSECUTIVE_RUNS, CL::End.get());
            let mut grouped_measurements = GroupMeasurements::new();
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPMC_RING_SIZE>(RingName::SPMCLoadBalancerCopy, &mut grouped_measurements); // Enforces Copy trait on payload
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPMC_RING_SIZE>(RingName::SPMCBroadcaster, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPMC_RING_SIZE>(RingName::SPMCBroadcasterPadded, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPMC_RING_SIZE>(RingName::SPMCBroadcasterUnsafeLocalTails, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, SPMC_RING_SIZE>(RingName::SPMCBroadcasterUnsafeLocalTailsOutsideArc, &mut grouped_measurements);
            // .. add more SPMC variants here
            grouped_measurements.print_relative_results();
        }


        // =-= MPSC =-= //
        if RUN_MPSC_EXPERIMENTS {
            println!("\n\n{}[*] ({}/{}) | Starting MPSC Ring Buffer Test{}\n", CL::Dull.get(), run_num + 1, CONSECUTIVE_RUNS, CL::End.get());
            let mut grouped_measurements = GroupMeasurements::new();
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, MPSC_RING_SIZE>(RingName::MPSCLocalTailLossy, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, MPSC_RING_SIZE>(RingName::MPSCLocalTailLossyPadded, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, MPSC_RING_SIZE>(RingName::MPSCGlobalTail, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, MPSC_RING_SIZE>(RingName::MPSCGlobalTailLossy, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, MPSC_RING_SIZE>(RingName::MPSCIndivSPSCGroup, &mut grouped_measurements);
            // .. add more MPSC variants here
            grouped_measurements.print_relative_results();
        }
        

        // =-= MPMC =-= //
        if RUN_MPMC_EXPERIMENTS {
            println!("\n\n{}[*] ({}/{}) | Starting MPMC Ring Buffer Test{}\n", CL::Dull.get(), run_num + 1, CONSECUTIVE_RUNS, CL::End.get());
            let mut grouped_measurements = GroupMeasurements::new();
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, MPMC_RING_SIZE>(RingName::MPMCLoadBalancer, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, MPMC_RING_SIZE>(RingName::MPMCLoadBalancerPadded, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, MPMC_RING_SIZE>(RingName::MPMCBroadcaster, &mut grouped_measurements);
            start_experiment::<BlanketPayloadChange<PAYLOAD_SIZE>, PAYLOAD_SIZE, MPMC_RING_SIZE>(RingName::MPMCBroadcasterUnsafeIndivSPMCCopy, &mut grouped_measurements); // Enforces Copy trait on payload
            // .. add more MPMC variants here
            grouped_measurements.print_relative_results();
        }
        

        println!("{}[*] ({}/{}) | All experiments complete!{}\n\n", CL::LimeGreen.get(), run_num + 1, CONSECUTIVE_RUNS, CL::End.get());
    }

    if FULL_SUITE_TESTING {
        run_full_suite_testing();
    }

}

