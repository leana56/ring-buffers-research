use super::*;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::sync::Barrier;


// =----------------------------------------------------= //

pub fn start_experiment<P: AnyPayload, const PAYLOAD_SIZE: usize, const RING_SIZE: usize>(ring_name: RingName, grouped_measurements: &mut GroupMeasurements) {
    match ring_name {
        // =-= MPMC =-= //
        RingName::MPMCLoadBalancer => {
            let (tx, rx) = MPMCLoadBalancer::<P, RING_SIZE>::new();
            run(tx, rx, MPMC_NUM_PRODUCERS, MPMC_NUM_CONSUMERS, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::MPMCLoadBalancerPadded => {
            let (tx, rx) = MPMCLoadBalancerPadded::<P, RING_SIZE>::new();
            run(tx, rx, MPMC_NUM_PRODUCERS, MPMC_NUM_CONSUMERS, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::MPMCBroadcaster => {
            let (tx, rx) = MPMCBroadcaster::<P, RING_SIZE>::new();
            run(tx, rx, MPMC_NUM_PRODUCERS, MPMC_NUM_CONSUMERS, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::MPMCBroadcasterUnsafeIndivSPMCCopy => {
            if P::COPYABLE {
                let (tx, rx) = MPMCBroadcasterUnsafeIndivSPMCCopy::<CopyablePayload<PAYLOAD_SIZE>, RING_SIZE>::new(MPMC_NUM_PRODUCERS); // Copy is enforced for this ring
                run(tx, rx, MPMC_NUM_PRODUCERS, MPMC_NUM_CONSUMERS, ring_name, RING_SIZE, grouped_measurements);
            } else {
                println!("{}[!][Experiment] Payload is not copyable, skipping ring: {:?}{}", CL::Dull.get(), ring_name, CL::End.get());
            }
        },
        // .. add more MPMC variants here


        // =-= MPSC =-= //
        RingName::MPSCLocalTailLossy => {
            let (tx, rx) = MPSCLocalTailLossy::<P, RING_SIZE>::new();
            run(tx, rx, MPSC_NUM_PRODUCERS, 1, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::MPSCLocalTailLossyPadded => {
            let (tx, rx) = MPSCLocalTailLossyPadded::<P, RING_SIZE>::new();
            run(tx, rx, MPSC_NUM_PRODUCERS, 1, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::MPSCGlobalTail => {
            let (tx, rx) = MPSCGlobalTail::<P, RING_SIZE>::new();
            run(tx, rx, MPSC_NUM_PRODUCERS, 1, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::MPSCGlobalTailLossy => {
            let (tx, rx) = MPSCGlobalTailLossy::<P, RING_SIZE>::new();
            run(tx, rx, MPSC_NUM_PRODUCERS, 1, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::MPSCIndivSPSCGroup => {
            let (tx, rx) = MPSCIndivSPSCGroup::<P, RING_SIZE>::new(MPSC_NUM_PRODUCERS);
            run(tx, rx, MPSC_NUM_PRODUCERS, 1, ring_name, RING_SIZE, grouped_measurements);
        },
        // .. add more MPSC variants here


        // =-= SPMC =-= //
        RingName::SPMCLoadBalancerCopy => {
            if P::COPYABLE {
                let (tx, rx) = SPMCLoadBalancerCopy::<CopyablePayload<PAYLOAD_SIZE>, RING_SIZE>::new();
                run(tx, rx, 1, SPMC_NUM_CONSUMERS, ring_name, RING_SIZE, grouped_measurements);
            } else {
                println!("{}[!][Experiment] Payload is not copyable, skipping ring: {:?}{}", CL::Dull.get(), ring_name, CL::End.get());
            }
        },
        RingName::SPMCBroadcaster => {
            let (tx, rx) = SPMCBroadcaster::<P, RING_SIZE>::new();
            run(tx, rx, 1, SPMC_NUM_CONSUMERS, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::SPMCBroadcasterPadded => {
            let (tx, rx) = SPMCBroadcasterPadded::<P, RING_SIZE>::new();
            run(tx, rx, 1, SPMC_NUM_CONSUMERS, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::SPMCBroadcasterUnsafeLocalTails => {
            let (tx, rx) = SPMCBroadcasterUnsafeLocalTails::<P, RING_SIZE>::new();
            run(tx, rx, 1, SPMC_NUM_CONSUMERS, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::SPMCBroadcasterUnsafeLocalTailsOutsideArc => {
            let ring = SPMCBroadcasterUnsafeLocalTailsOutsideArc::<P, RING_SIZE>::new();
            run(Arc::clone(&ring), Arc::clone(&ring), 1, SPMC_NUM_CONSUMERS, ring_name, RING_SIZE, grouped_measurements);
        },
        // .. add more SPMC variants here


        // =-= SPSC =-= //
        RingName::SPSCDualIndexFalseSharing => {
            let (tx, rx) = SPSCDualIndexFalseSharing::<P, RING_SIZE>::new();
            run(tx, rx, 1, 1, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::SPSCDualIndexPadFalseSharing => {
            let (tx, rx) = SPSCDualIndexPadFalseSharing::<P, RING_SIZE>::new();
            run(tx, rx, 1, 1, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::SPSCSafeSkipping => {
            let (tx, rx) = SPSCSafeSkipping::<P, RING_SIZE>::new();
            run(tx, rx, 1, 1, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::SPSCSafeSkippingNoBoxPtr => {
            let (tx, rx) = SPSCSafeSkippingNoBoxPtr::<P, RING_SIZE>::new();
            run(tx, rx, 1, 1, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::SPSCSafeSkippingOutsideArc => {
            let ring = SPSCSafeSkippingOutsideArc::<P, RING_SIZE>::new();
            run(Arc::clone(&ring), Arc::clone(&ring), 1, 1, ring_name, RING_SIZE, grouped_measurements);
        },
        RingName::SPSCSlotLockLocalTailCopy => {
            if P::COPYABLE {
                let (tx, rx) = SPSCSlotLockLocalTailCopy::<CopyablePayload<PAYLOAD_SIZE>, RING_SIZE>::new();
                run(tx, rx, 1, 1, ring_name, RING_SIZE, grouped_measurements);
            } else {
                println!("{}[!][Experiment] Payload is not copyable, skipping ring: {:?}{}", CL::Dull.get(), ring_name, CL::End.get());
            }
        },
        RingName::SPSCFullLockLocalTailCopy => {
            if P::COPYABLE {
                let (tx, rx) = SPSCFullLockLocalTailCopy::<CopyablePayload<PAYLOAD_SIZE>, RING_SIZE>::new();
                run(tx, rx, 1, 1, ring_name, RING_SIZE, grouped_measurements);
            } else {
                println!("{}[!][Experiment] Payload is not copyable, skipping ring: {:?}{}", CL::Dull.get(), ring_name, CL::End.get());
            }
        },
        // .. add more SPSC variants here


        _ => panic!("Invalid ring name: {:?}", ring_name),
    }
}


// =----------------------------------------------------= //


fn run<H, E, Y>( //, World!
    tx: H, 
    rx: E, 
    producer_threads: usize,
    consumer_threads: usize,
    ring_name: RingName,
    ring_size: usize,
    grouped_measurements: &mut GroupMeasurements
) where 
    H: Sender<Y> + Clone + Send + 'static,
    E: Receiver<Y> + Clone + Send + 'static,
    Y: AnyPayload + Send + 'static
{
    println!("{}=--------------------------= {:?} =--------------------------={}", CL::Green.get(), ring_name, CL::End.get());

    let mut threads = Vec::new();

    let start_barrier = Arc::new(Barrier::new(producer_threads + consumer_threads));
    let exit_signal = Arc::new(AtomicBool::new(false));

    // =------------------------------------------------------------------= //
    // =-= Producer Threads =-= //

    for producer_thread in 0..producer_threads {
        let tx = tx.clone();
        let start_barrier_clone = Arc::clone(&start_barrier);
        let exit_signal_clone = Arc::clone(&exit_signal);

        let thread = std::thread::Builder::new().spawn(move || {
            if !core_affinity::set_for_current(core_affinity::CoreId { id: PRODUCER_PIN_CPU_IDS[producer_thread] }) {
                panic!("{}[!][Producer-{}] Failed to pin to core{}", CL::Red.get(), producer_thread, CL::End.get());
            }


            // =-= Producer =-= //
            let idx_offset = SAMPLE_SIZE * producer_thread;
            let payload_stash = PayloadStash::<Y>::new(ring_name, consumer_threads, idx_offset);

            start_barrier_clone.wait();
            for mut payload in payload_stash.payloads {
                payload.update_timestamp();
                tx.push(producer_thread, payload);

                if BURN_PRODUCER_TIME > 0 {
                    burn(Duration::from_micros(BURN_PRODUCER_TIME));
                }

                if exit_signal_clone.load(Ordering::Relaxed) {
                    break;
                }
            }
        });

        match thread {
            Ok(thread) => threads.push(thread),
            Err(e) => panic!("[!][Producer-{}] Failed to spawn thread: {}", producer_thread, e),
        }
    }

    // =------------------------------------------------------------------= //
    // =-= Consumer Threads =-= //

    let barrier = Arc::new(Barrier::new(consumer_threads));
    let (measurements_tx, measurements_rx) = std::sync::mpsc::channel();

    for consumer_thread in 0..consumer_threads {
        let rx = rx.clone();
        let barrier_clone = Arc::clone(&barrier);
        let start_barrier_clone = Arc::clone(&start_barrier);
        let measurements_tx_clone = measurements_tx.clone();

        let thread = std::thread::Builder::new().spawn(move || {
            // =-= CPU Pinning =-= //
            if !core_affinity::set_for_current(core_affinity::CoreId { id: CONSUMER_PIN_CPU_IDS[consumer_thread] }) {
                panic!("{}[!][Consumer-{}] Failed to pin to core{}", CL::Red.get(), consumer_thread, CL::End.get());
            }

            // =-= Consumer =-= //
            let mut measurements = IndividualMeasurements::<Y>::new(ring_name, ring_size, consumer_thread, consumer_threads, producer_threads);
            start_barrier_clone.wait();
            measurements.start();

            let mut local_tail = 0;
            loop {
                let payload = rx.pop(&mut local_tail);
                if measurements.add(payload.collapse_timestamp()) {
                    break;  // We've received all the samples
                }

                if BURN_CONSUMER_TIME > 0 {
                    burn(Duration::from_micros(BURN_CONSUMER_TIME));
                }
            }

            measurements.stop();
            barrier_clone.wait();
            measurements_tx_clone.send(measurements).expect(format!("[!][Consumer-{}] Failed to send measurements", consumer_thread).as_str());
        });

        match thread {
            Ok(thread) => threads.push(thread),
            Err(e) => panic!("[!][Consumer-{}] Failed to spawn thread: {}", consumer_thread, e),
        }
    }

    drop(measurements_tx);
    for _ in 0..consumer_threads {
        if let Ok(mut measurements) = measurements_rx.recv() {
            let dumped_results = measurements.print_and_dump_stats();
            grouped_measurements.add(dumped_results);
        }
    }

    exit_signal.store(true, Ordering::Relaxed);
    for thread in threads {
        thread.join().unwrap();
    }
}

#[inline]
fn burn(duration: Duration) {
    let start = minstant::Instant::now();
    while start.elapsed() < duration {
        core::hint::spin_loop();
    }
}