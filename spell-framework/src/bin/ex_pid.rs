use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

use fs2::FileExt;
use signal_hook::consts::*;
use signal_hook::flag;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pid_file_path = "/tmp/myprogram.pid";

    // Open (or create) PID file
    let mut pid_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(pid_file_path)?;

    // Try locking the file exclusively
    if let Err(_) = pid_file.try_lock_exclusive() {
        eprintln!("Another instance is already running!");
        std::process::exit(1);
    }

    // Write our PID into the file
    pid_file.set_len(0)?; // clear old contents
    write!(pid_file, "{}", std::process::id())?;
    pid_file.sync_all()?;

    // Shared ON/OFF flag
    let running = Arc::new(AtomicBool::new(false));
    let r_on = Arc::clone(&running);
    let r_off = Arc::clone(&running);

    // Register signals
    flag::register(SIGUSR1, r_on.clone())?; // Turn ON
    flag::register(SIGUSR2, r_off.clone())?; // Turn OFF

    println!("Process started. PID: {}", std::process::id());
    println!("Send SIGUSR1 to turn ON, SIGUSR2 to turn OFF.");
    println!("PID file: {}", pid_file_path);

    // Main loop
    loop {
        if running.load(Ordering::Relaxed) {
            println!("Running...");
        } else {
            println!("Stopped.");
        }
        thread::sleep(Duration::from_secs(1));
    }
}
