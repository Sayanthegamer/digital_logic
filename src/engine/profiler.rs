use rayon::prelude::*;
use std::time::Instant;
use std::sync::OnceLock;

static CROSSOVER_THRESHOLD: OnceLock<usize> = OnceLock::new();

/// Dynamically tests the host machine to find the optimal
/// crossover point where parallel evaluation becomes faster than sequential.
pub fn detect_parallel_crossover_threshold() -> usize {
    *CROSSOVER_THRESHOLD.get_or_init(|| {
        // Warmup Rayon threadpool
        let _ = (0..1000).into_par_iter().map(|x| x * 2).collect::<Vec<_>>();

        let sizes = [100, 500, 1000, 2000, 4000, 8000, 16000];
        
        for &size in &sizes {
            let states: Vec<u8> = (0..size).map(|i| (i % 4) as u8).collect();
            let queue: Vec<usize> = (0..size).collect();

            let mut seq_time = std::time::Duration::from_secs(100);
            let mut par_time = std::time::Duration::from_secs(100);

            for _ in 0..5 { // 5 samples to reduce noise
                let start = Instant::now();
                let seq_res = queue.iter().filter_map(|&idx| {
                    let val_a = states[idx];
                    let val_b = states[(idx + 1) % size];
                    let a_bool = (val_a & 0b10) != 0;
                    let b_bool = (val_b & 0b10) != 0;
                    let new_state = if !(a_bool && b_bool) { 0b10 } else { 0b01 };
                    if new_state != val_a { Some((idx, new_state)) } else { None }
                }).collect::<Vec<_>>();
                seq_time = seq_time.min(start.elapsed());
                // Prevent optimization
                assert!(seq_res.len() <= size);

                let start = Instant::now();
                let par_res = queue.par_iter().filter_map(|&idx| {
                    let val_a = states[idx];
                    let val_b = states[(idx + 1) % size];
                    let a_bool = (val_a & 0b10) != 0;
                    let b_bool = (val_b & 0b10) != 0;
                    let new_state = if !(a_bool && b_bool) { 0b10 } else { 0b01 };
                    if new_state != val_a { Some((idx, new_state)) } else { None }
                }).collect::<Vec<_>>();
                par_time = par_time.min(start.elapsed());
                assert!(par_res.len() <= size);
            }

            if par_time < seq_time {
                println!("Hardware Calibration: Rayon parallelization threshold set at {} gates", size);
                return size;
            }
        }
        
        // Fallback if parallel is somehow slower even at 16000 gates
        println!("Hardware Calibration: Rayon parallelization threshold set at 10000 gates (Fallback)");
        10000
    })
}
