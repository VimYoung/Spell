use std::process::{Command, Stdio};

fn main() {
    let output = Command::new("sh")
        .arg("-c")
        .arg("last | awk '{print $1}' | sort | uniq -c | sort -nr")
        .output()
        .expect("failed to run pipeline");

    let val = String::from_utf8_lossy(&output.stdout);
    let val_2 = val.split('\n').collect::<Vec<_>>()[0].trim();
    let username = val_2.split(" ").collect::<Vec<_>>()[1];

    println!("/{}/", username);
}
