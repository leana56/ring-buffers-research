use crate::IndividualMeasurementsDumped;
use crate::CL;

// =------------------------------------------------= //
// =-= Group Measurements =-= //
// - To help with the analsyis, after all individual measurements belonging to a specific channel type have been complete, we'll print out their respective relative results to the avg baseline across all of them
// =------------------------------------------------= //


pub struct GroupMeasurements {
    pub measurements: Vec<IndividualMeasurementsDumped>,
}

impl GroupMeasurements {
    pub fn new() -> Self {
        Self { 
            measurements: Vec::new() 
        }
    }
    
    pub fn add(&mut self, measurements: IndividualMeasurementsDumped) {
        self.measurements.push(measurements);
    }

    pub fn print_relative_results(&self) {
        let n = self.measurements.len() as f64;
        let mean = |f: fn(&IndividualMeasurementsDumped) -> f64| {
            self.measurements.iter().map(f).sum::<f64>() / n
        };

        let bl_elapsed_ns = mean(|m| m.elapsed_time_ns);
        let bl_msgs_s = mean(|m| m.throughput_msgs_per_s);
        let bl_bytes_s = mean(|m| m.bytes_throughput_per_s);

        // =-= Print Results =-= //
        let ring_name_padding = self.measurements.iter().map(|m| format!("{} ({})", m.ring_name.to_string(), m.consumer_id).len()).max().unwrap_or(2); 
        println!("\n{:<ring_name_padding$} | {:>8} | {:>8} | {:>8} | {:>6} | {:>6}", "Ring Name", "El.", "Msg", "Bps", "DL%", "DP%");
        println!("{}{:-<ring_name_padding$}{:-<53}{}", CL::Teal.get(), "", "", CL::End.get());

        fn decorate(val: f64, good_is_low: bool) -> String {
            let arrow = if (val < 0.0) == good_is_low { " ▲" } else { " ▼" };
            let s = format!("{:>6.1}{}", val, arrow);

            if (val < 0.0) == good_is_low {
                format!("{}{}{}", CL::Green.get(), s, CL::End.get())
            } else {
                format!("{}{}{}", CL::Red.get(), s, CL::End.get())
            }
        }

        for m in &self.measurements {
            let pct = |val, base| (val - base) / base * 100.0;
            let elapsed_time_ns = decorate(pct(m.elapsed_time_ns,  bl_elapsed_ns),  true);
            let throughput_msgs_per_s = decorate(pct(m.throughput_msgs_per_s, bl_msgs_s), false);
            let bytes_throughput_per_s = decorate(pct(m.bytes_throughput_per_s, bl_bytes_s), false);

            let dl_cell = format!("{:>6.2}", m.data_loss_percentage);
            let dp_cell = format!("{:>6.2}", m.duplicates_percentage); 

            let rn = format!("{} ({})", m.ring_name.to_string(), m.consumer_id);
            let short = if rn.len() > ring_name_padding { &rn[..ring_name_padding] } else { &rn };
            let name = format!("{:<ring_name_padding$}", short);
            println!("{} | {} | {} | {} | {} | {}", name, elapsed_time_ns, throughput_msgs_per_s, bytes_throughput_per_s, dl_cell, dp_cell);
        }

        println!("{}{:-<ring_name_padding$}{:-<53}{}", CL::Teal.get(), "", "", CL::End.get());

    }
}
