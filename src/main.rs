use std::{
    thread,
    time::{self, Duration},
};

fn main() {
    // Theory: Split cycles into time slices of work as to play nice with non-realtime OS.
    let start = time::Instant::now();
    for time_step in 1..=1000 {
        // Do work
        // let after = time::Instant::now();
        let expected_time = start + Duration::from_millis(time_step);
        let delay = expected_time - time::Instant::now();
        println!("{:?} {:?}", delay, expected_time);
        // let before = time::Instant::now();
        thread::sleep(delay);
        // println!("{:?} - {:?} = {:?}", after, before, after - before);
    }
    let end = time::Instant::now();
    println!("Expected time step: 100ms, actual: {:?}", end - start);
}
