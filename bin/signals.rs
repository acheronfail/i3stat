use libc::{SIGRTMAX, SIGRTMIN};
use serde_json::json;

fn main() {
    println!(
        "{}",
        json!({ "sigrtmin": SIGRTMIN(), "sigrtmax": SIGRTMAX() })
    );
}
