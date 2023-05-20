use libc::{SIGRTMAX, SIGRTMIN};
use serde_json::json;

fn main() {
    let rt_min = SIGRTMIN();
    let rt_max = SIGRTMAX();
    println!(
        "{}",
        json!({
            "min": 0,
            "max": rt_max - rt_min,
            "sigrtmin": rt_min,
            "sigrtmax": rt_max
        })
    );
}
