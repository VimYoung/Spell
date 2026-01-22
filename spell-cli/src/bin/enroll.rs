use std::sync::{Arc, Mutex};

use libfprint_rs::{FpContext, FpDevice, FpFinger, FpPrint};

fn main() {
    // Get context
    let ctx = FpContext::new();
    // Collect connected devices
    let devices = ctx.devices();

    // Get the first connected device
    let dev = devices.first().unwrap();

    // Open the device to start operations
    dev.open_sync(None).unwrap();

    // Create a template print
    let template = FpPrint::new(dev);
    template.set_finger(FpFinger::RightRing);
    template.set_username("test");

    // User data that we will use on the callback function,
    // to mutate the value of a counter, it must be wrapped in an Arc<Mutex<T>>
    let counter = Arc::new(Mutex::new(0));

    // Get the new print from the user
    let _new_print = dev
        .enroll_sync(template, None, Some(progress_cb), Some(counter.clone()))
        .unwrap();

    // Get the total of time the enroll callback was called
    println!("Total enroll stages: {}", counter.lock().unwrap());
}

pub fn progress_cb(
    _device: &FpDevice,
    enroll_stage: i32,
    _print: Option<FpPrint>,
    _error: Option<libfprint_rs::GError>,
    data: &Option<Arc<Mutex<i32>>>,
) {
    if let Some(data) = data {
        let mut d = data.lock().unwrap();
        *d += 1;
    }
    println!("Enroll stage: {}", enroll_stage);
}
