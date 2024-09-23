use inbot::*;

fn main() {
    let mut listener_proxy = start_listen();
    let key = BindingKey {
        key: KeyCode::KeyA,
        modifer_keys: vec![],
    };
    listener_proxy.bind_once(
        vec![key],
        Box::new(|| {
            println!("Key `A` Triggered!");
        }),
    );

    let key = BindingKey {
        key: KeyCode::KeyF,
        modifer_keys: vec![KeyCode::ControlLeft],
    };
    listener_proxy.bind_multi(
        vec![key],
        Box::new(|| {
            println!("Key `Ctrl +  F` Triggered!");
        }),
    );
    let key1 = BindingKey {
        key: KeyCode::KeyK,
        modifer_keys: vec![KeyCode::ControlLeft, KeyCode::ShiftLeft],
    };
    let key2 = BindingKey {
        key: KeyCode::KeyC,
        modifer_keys: vec![KeyCode::ControlLeft, KeyCode::ShiftLeft],
    };
    let keep_running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let keep_running_copy = keep_running.clone();
    listener_proxy.bind_multi(
        vec![key1, key2],
        Box::new(move || {
            println!("Key `Ctrl + Shift + K + C` Triggered");
            keep_running_copy.swap(false, std::sync::atomic::Ordering::SeqCst);
        }),
    );
    while keep_running.load(std::sync::atomic::Ordering::Relaxed) {
        listener_proxy.update();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    stop_listen();
}
