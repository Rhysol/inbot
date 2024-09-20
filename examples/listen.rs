use inbot::*;

fn main() {
    let _ = start_listen();
    std::thread::sleep(std::time::Duration::from_secs(100));
    // stop_listen();
}
