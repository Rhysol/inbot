use crate::binding_key_mgr::BindingKeyMgr;
use crate::virtual_key::*;
use std::ptr::null_mut;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::thread::{self, JoinHandle};
use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, PeekMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK, MSG,
    PM_REMOVE, WH_KEYBOARD_LL, WH_MOUSE_LL,
};

static mut LISTENER: OnceLock<Listener> = OnceLock::new();
static LISTENER_PROXY: Mutex<Option<ListenerProxy>> = Mutex::new(None);
static mut KEYBOARD_HOOK: HHOOK = HHOOK(null_mut());
static mut MOUSE_HOOK: HHOOK = HHOOK(null_mut());

pub fn start_listen() -> ListenerProxy {
    let mut proxy_guard = LISTENER_PROXY.lock().unwrap();
    if let Some(proxy) = proxy_guard.as_ref() {
        return proxy.clone();
    }
    let keep_listening = Arc::new(AtomicBool::new(true));
    let keep_listening_copy = keep_listening.clone();
    let join_handle = thread::spawn(move || unsafe {
        LISTENER.get_or_init(|| Listener::new(keep_listening_copy));
        let listener = LISTENER.get_mut().unwrap();
        listener.thread_loop();
    });
    let proxy = ListenerProxy {
        listener_thread: Arc::new(Mutex::new(Some(join_handle))),
        keep_listening,
    };
    proxy_guard.replace(proxy.clone());
    proxy
}

pub fn stop_listen() {
    let proxy = LISTENER_PROXY.lock().unwrap();
    if let Some(proxy) = proxy.as_ref() {
        proxy.keep_listening.swap(false, Ordering::Relaxed);
        if let Some(join_handle) = proxy.listener_thread.lock().unwrap().take() {
            let _ = join_handle.join();
        }
    }
}

pub struct ListenerProxy {
    listener_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    keep_listening: Arc<AtomicBool>,
}

impl ListenerProxy {
    fn clone(&self) -> Self {
        Self {
            listener_thread: self.listener_thread.clone(),
            keep_listening: self.keep_listening.clone(),
        }
    }
}

struct Listener {
    keep_listening: Arc<AtomicBool>,
    binding_key_mgr: BindingKeyMgr,
}

impl Listener {
    fn new(keep_listening: Arc<AtomicBool>) -> Self {
        Self {
            keep_listening,
            binding_key_mgr: BindingKeyMgr::new(),
        }
    }

    unsafe fn thread_loop(&mut self) {
        KEYBOARD_HOOK = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_callback),
            HMODULE::default(),
            0,
        )
        .unwrap();
        MOUSE_HOOK =
            SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_callback), HMODULE::default(), 0).unwrap();
        let mut msg = MSG::default();
        while self.keep_listening.load(Ordering::Relaxed) {
            match PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).0 {
                -1 => {
                    println!("get message failed");
                    return;
                }
                0 => std::thread::sleep(std::time::Duration::from_millis(10)),
                _ => (),
            };
        }
        let _ = UnhookWindowsHookEx(KEYBOARD_HOOK);
        let _ = UnhookWindowsHookEx(MOUSE_HOOK);
    }
}

extern "system" fn keyboard_callback(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { device_event_callback(KEYBOARD_HOOK, ncode, wparam, lparam) }
}

extern "system" fn mouse_callback(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { device_event_callback(MOUSE_HOOK, ncode, wparam, lparam) }
}

unsafe fn device_event_callback(
    hook: HHOOK,
    ncode: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if ncode == HC_ACTION as i32 {
        let listener = LISTENER.get_mut().unwrap();
        if let Some(input_key) = InputKey::from(wparam, lparam) {
            listener.binding_key_mgr.on_input_key(input_key);
        } else {
            println!("parse event:{} failed", wparam.0);
        }
    }
    CallNextHookEx(hook, ncode, wparam, lparam)
}
