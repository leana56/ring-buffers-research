use crate::MEASUREMENT_SAMPLE_PERCENTAGE;
use crate::SAVE_MEASUREMENTS;
use crate::PAYLOAD_BYTE_TYPE;
use crate::PAYLOAD_STASH_GEN_ON_FLY;
use crate::BURN_PRODUCER_TIME;
use crate::BURN_CONSUMER_TIME;
use crate::SAMPLE_SIZE;
use crate::PayloadByteType;
use crate::FileHandler;
use crate::AnyPayload;
use crate::RingName;
use crate::CL;

use serde::Serialize;

// =------------------------------------------------= //
// =-= Measurements =-= //
// - This struct is a smol wrapper around the mechanisms for measuring the performance of any ring buffer,
// - It measures and keeps track of: Throughput, Latency, and Data Loss
// - Important distinction: timestamp includes the queueing time, not just the processing time (which could lead to lower bytes looking "slower" on an individual basis, tho faster when looking at total elapsed time)
//   - This is due to the fact the producer might be faster than the consumer, or vice versa, which shows up in these measurements

pub struct IndividualMeasurements<P: AnyPayload> {
    pub ring_name: RingName,
    pub ring_size: usize,
    pub consumer_id: usize,
    pub producer_threads: usize,
    pub consumer_threads: usize,
    pub start_time: minstant::Instant,
    pub capacity: usize,
    pub samples: Box<[(usize, u128)]>,
    pub samples_index: usize,
    pub sample_seq: usize,
    pub next_sample: usize,
    pub sample_rate: usize,
    pub termination_count: usize,
    pub total_elapsed_nanos: u128,
    pub save_results: FileHandler,
    pub _marker: std::marker::PhantomData<P>,
}

impl<P: AnyPayload> IndividualMeasurements<P> {
    pub fn new(
        ring_name: RingName, 
        ring_size: usize,
        consumer_id: usize,
        consumer_threads: usize,
        producer_threads: usize,
    ) -> Self {
        let capacity = (SAMPLE_SIZE * producer_threads * MEASUREMENT_SAMPLE_PERCENTAGE + 99) / 100;
        let sample_rate = 100 / MEASUREMENT_SAMPLE_PERCENTAGE.max(1);
        Self { 
            ring_name, 
            ring_size,
            consumer_id,
            consumer_threads,
            producer_threads,
            start_time: minstant::Instant::now(),
            capacity,
            samples: (0..capacity).map(|_| (0, 0)).collect::<Vec<(usize, u128)>>().into_boxed_slice(),
            samples_index: 0,
            sample_seq: 0,
            next_sample: sample_rate,
            sample_rate,
            termination_count: 0,
            total_elapsed_nanos: 0,
            save_results: FileHandler::new(&format!("results/{:?}/{:?}.txt", ring_name.get_channel_type(), ring_name)).unwrap(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn start(&mut self) {
        self.start_time = minstant::Instant::now();
    }

    #[inline]
    pub fn add(&mut self, (idx, elapsed): (usize, u128)) -> bool {
        if idx == usize::MAX {
            self.termination_count += 1;
            return self.termination_count >= self.producer_threads;
        }
        
        if self.samples_index >= self.capacity { return true; }

        self.sample_seq += 1;
        if self.sample_seq == self.next_sample {
            unsafe { *self.samples.get_unchecked_mut(self.samples_index) = (idx, elapsed); }
            self.samples_index += 1;
            self.next_sample = self.sample_seq + self.sample_rate;
        }

        false
    }

    pub fn stop(&mut self) {
        self.total_elapsed_nanos = self.start_time.elapsed().as_nanos();
    }

    pub fn print_and_dump_stats(&mut self) -> IndividualMeasurementsDumped {

        // =------------------------------------------------------------------= //

        self.samples.sort_by_key(|(_, elapsed)| *elapsed);

        let non_zero_samples: Vec<&(usize, u128)> = self.samples.iter().filter(|(idx, _)| *idx > 0).collect();
        let sample_count = non_zero_samples.len();

        let (min_sample, max_sample, median_sample, avg_sample) = match sample_count == 0 { // "use an if statement for less horizontal space" but match looks so cleann
            true => (0, 0, 0, 0),
            false => {
                let min_sample = non_zero_samples[0].1;
                let max_sample = non_zero_samples[sample_count - 1].1;
                let median_sample = non_zero_samples[sample_count / 2].1;
                let avg_sample = non_zero_samples.iter().map(|(_, elapsed)| elapsed).sum::<u128>() / sample_count as u128;
                (min_sample, max_sample, median_sample, avg_sample)
            }
        };

        let mut duplicates = 0;
        let mut seen = vec![false; (SAMPLE_SIZE * self.producer_threads) + 1];
        for (idx, _) in self.samples.iter() {
            if *idx == 0 { continue; }

            if seen[*idx] {
                duplicates += 1;
            } else {
                seen[*idx] = true;
            }
        }
        let received = seen.iter().filter(|x| **x).count();
        let lost = self.capacity - received - 1;

        let duplicates_percentage = (duplicates as f64 / self.capacity as f64) * 100.0;
        let lost_samples_percentage = (lost as f64 / self.capacity as f64) * 100.0;

        // =------------------------------------------------------------------= //

        const NANOS_PER_MICRO: f64  = 1_000.0;
        const NANOS_PER_MILLI: f64  = 1_000_000.0;
        const NANOS_PER_SECOND: f64 = 1_000_000_000.0;

        let total_elapsed_ns     = self.total_elapsed_nanos as f64;
        let total_elapsed_us     = total_elapsed_ns / NANOS_PER_MICRO;
        let total_elapsed_ms     = total_elapsed_ns / NANOS_PER_MILLI;
        let total_elapsed_sec    = total_elapsed_ns / NANOS_PER_SECOND;

        let msgs_per_ns = SAMPLE_SIZE as f64 / total_elapsed_ns;
        let msgs_per_us = SAMPLE_SIZE as f64 / total_elapsed_us;
        let msgs_per_ms = SAMPLE_SIZE as f64 / total_elapsed_ms;
        let msgs_per_s  = SAMPLE_SIZE as f64 / total_elapsed_sec;

        let total_bytes = P::PAYLOAD_SIZE as f64 * SAMPLE_SIZE as f64;
        let bytes_per_ns = total_bytes / total_elapsed_ns;
        let bytes_per_us = total_bytes / total_elapsed_us;
        let bytes_per_ms = total_bytes / total_elapsed_ms;
        let bytes_per_s  = total_bytes / total_elapsed_sec;

        // =--------------------------------------------------------------= //
        // =-= Formatting & Printing =-= //

        let elapsed_row = [
            format!("{:.0}", total_elapsed_ns),
            format!("{:.0}", total_elapsed_us),
            format!("{:.2}", total_elapsed_ms),
            format!("{:.4}", total_elapsed_sec),
        ];
        let throughput_row = [
            format!("{:.3}", msgs_per_ns),
            format!("{:.0}", msgs_per_us),
            format!("{:.0}", msgs_per_ms),
            format!("{:.0}", msgs_per_s),
        ];
        let bytes_row = [
            format!("{:.3}", bytes_per_ns),
            format!("{:.0}", bytes_per_us),
            format!("{:.0}", bytes_per_ms),
            format!("{:.0}", bytes_per_s),
        ];

        let rows = [
            ("Elapsed Time", &elapsed_row),
            ("Throughput (msgs/)", &throughput_row),
            ("Bytes Throughput (B/)", &bytes_row),
        ];

        let mut col_width = [0usize; 4];
        let mut hdr_width = 0usize;

        for (hdr, cells) in &rows {
            hdr_width = hdr_width.max(hdr.len());
            for i in 0..4 {
                col_width[i] = col_width[i].max(cells[i].len());
            }
        }

        fn pad_left(s: &str, width: usize) -> String {
            let mut out = String::with_capacity(width);
            for _ in 0..(width - s.len()) { out.push(' '); }
            out.push_str(s);
            out
        }
        fn pad_right(s: &str, width: usize) -> String {
            let mut out = String::with_capacity(width);
            out.push_str(s);
            for _ in 0..(width - s.len()) { out.push(' '); }
            out
        }

        // =------------------------------------------------------------------= //

        println!("{}[*] Results for consumer thread: {}{}", CL::Teal.get(), self.consumer_id, CL::End.get());
        for (hdr, cells) in &rows {
            println!("{} |:| {} ns | {} µs | {} ms | {} s", pad_right(hdr, hdr_width), pad_left(&cells[0], col_width[0]), pad_left(&cells[1], col_width[1]), pad_left(&cells[2], col_width[2]), pad_left(&cells[3], col_width[3]));
        }

        println!("{}Latency |:| Min: {} ns | Max: {} ns | Median: {} ns | Avg: {} ns{}", CL::Dull.get(), min_sample, max_sample, median_sample, avg_sample, CL::End.get());

        let health_color = if lost_samples_percentage > 0.0 || duplicates_percentage > 0.0 { CL::Orange.get() } else { CL::Dull.get() };
        println!("{}Health |:|{} {}Data Loss: {:.2}% | Duplicates: {:.2}%{}", CL::Dull.get(), CL::End.get(), health_color, lost_samples_percentage, duplicates_percentage, CL::End.get());

        let results = IndividualMeasurementsDumped {
            ring_name: self.ring_name,
            ring_size: self.ring_size,
            consumer_id: self.consumer_id,
            sample_size: SAMPLE_SIZE,
            copyable_payload: P::COPYABLE,
            payload_size: P::PAYLOAD_SIZE,
            payload_type: PAYLOAD_BYTE_TYPE,
            payload_stash_gen_on_fly: PAYLOAD_STASH_GEN_ON_FLY,
            burn_producer_time: BURN_PRODUCER_TIME,
            burn_consumer_time: BURN_CONSUMER_TIME,
            measurement_sample_percentage: MEASUREMENT_SAMPLE_PERCENTAGE,
            producer_threads: self.producer_threads,
            consumer_threads: self.consumer_threads,
            elapsed_time_ns: total_elapsed_ns,
            elapsed_time_us: total_elapsed_us,
            elapsed_time_ms: total_elapsed_ms,
            elapsed_time_s: total_elapsed_sec,
            throughput_msgs_per_ns: msgs_per_ns,
            throughput_msgs_per_us: msgs_per_us,
            throughput_msgs_per_ms: msgs_per_ms,
            throughput_msgs_per_s: msgs_per_s,
            bytes_throughput_per_ns: bytes_per_ns,
            bytes_throughput_per_us: bytes_per_us,
            bytes_throughput_per_ms: bytes_per_ms,
            bytes_throughput_per_s: bytes_per_s,
            min_latency_ns: min_sample,
            max_latency_ns: max_sample,
            median_latency_ns: median_sample,
            avg_latency_ns: avg_sample,
            data_loss_percentage: lost_samples_percentage,
            duplicates_percentage: duplicates_percentage,
        };

        if SAVE_MEASUREMENTS {
            let results_str = serde_json::to_string(&results).unwrap();
            self.save_results.write_line(results_str).unwrap();
        }

        results
    }
}

// =------------------------------------------------= //

#[derive(Debug, Clone, Serialize)]
pub struct IndividualMeasurementsDumped {
    pub ring_name: RingName,
    pub ring_size: usize,
    pub consumer_id: usize,
    pub sample_size: usize,
    pub copyable_payload: bool,
    pub payload_size: usize,
    pub payload_type: PayloadByteType,
    pub payload_stash_gen_on_fly: bool,
    pub burn_producer_time: u64,
    pub burn_consumer_time: u64,
    pub measurement_sample_percentage: usize,
    pub producer_threads: usize,
    pub consumer_threads: usize,
    pub elapsed_time_ns: f64,
    pub elapsed_time_us: f64,
    pub elapsed_time_ms: f64,
    pub elapsed_time_s: f64,
    pub throughput_msgs_per_ns: f64,
    pub throughput_msgs_per_us: f64,
    pub throughput_msgs_per_ms: f64,
    pub throughput_msgs_per_s: f64,
    pub bytes_throughput_per_ns: f64,
    pub bytes_throughput_per_us: f64,
    pub bytes_throughput_per_ms: f64,
    pub bytes_throughput_per_s: f64,
    pub min_latency_ns: u128,
    pub max_latency_ns: u128,
    pub median_latency_ns: u128,
    pub avg_latency_ns: u128,
    pub data_loss_percentage: f64,
    pub duplicates_percentage: f64,
}