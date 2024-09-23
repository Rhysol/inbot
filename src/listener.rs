use crate::binding_key_mgr::*;
use crate::virtual_key::*;
use std::collections::HashMap;
use std::ptr::null_mut;
use std::sync::{
    mpsc::{channel, Receiver, Sender, TryRecvError},
    Mutex, OnceLock,
};
use std::thread::{self, JoinHandle};
use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, PeekMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK, MSG,
    PM_REMOVE, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_MOUSEMOVE,
};

static mut LISTENER: OnceLock<Listener> = OnceLock::new();
static LISTENER_THREAD: Mutex<OnceLock<JoinHandle<()>>> = Mutex::new(OnceLock::new());
static LISTENER_OPT_TX: OnceLock<Sender<ListenerOpt>> = OnceLock::new();
static mut KEYBOARD_HOOK: HHOOK = HHOOK(null_mut());
static mut MOUSE_HOOK: HHOOK = HHOOK(null_mut());

pub fn start_listen() -> ListenerProxy {
    if let Some(listener_opt_tx) = LISTENER_OPT_TX.get() {
        return ListenerProxy::new(listener_opt_tx.clone());
    }
    let (binding_opt_tx, binding_opt_rx) = channel();
    let join_handle = thread::spawn(move || unsafe {
        LISTENER.get_or_init(|| Listener::new(binding_opt_rx));
        let listener = LISTENER.get_mut().unwrap();
        listener.thread_loop();
    });
    LISTENER_THREAD
        .lock()
        .unwrap()
        .get_or_init(move || join_handle);
    let binding_opt_tx1 = binding_opt_tx.clone();
    LISTENER_OPT_TX.get_or_init(move || binding_opt_tx1);
    let proxy = ListenerProxy::new(binding_opt_tx);
    proxy
}

pub fn stop_listen() {
    if let Some(opt_tx) = LISTENER_OPT_TX.get() {
        let _ = opt_tx.send(ListenerOpt::StopListen);
    }
    if let Some(join_handle) = LISTENER_THREAD.lock().unwrap().take() {
        let _ = join_handle.join();
    }
}

enum ListenerOpt {
    Bind(BindingInfo),
    Unbind(u32),
    StopListen,
}

enum BindingCallback {
    Once(Box<dyn FnOnce() + Send + 'static>),
    Multi(Box<dyn FnMut()>),
}

pub struct ListenerProxy {
    binding_opt_tx: Sender<ListenerOpt>,
    binding_notifier_tx: Sender<u32>,
    binding_notifier_rx: Receiver<u32>,
    callbacks: HashMap<u32, BindingCallback>,
}

impl ListenerProxy {
    fn new(binding_opt_tx: Sender<ListenerOpt>) -> Self {
        let (binding_notifier_tx, binding_notifier_rx) = channel();
        Self {
            binding_opt_tx,
            binding_notifier_tx,
            binding_notifier_rx,
            callbacks: HashMap::new(),
        }
    }

    fn fork(&self) -> Self {
        let (binding_notifier_tx, binding_notifier_rx) = channel();
        Self {
            binding_opt_tx: self.binding_opt_tx.clone(),
            binding_notifier_tx,
            binding_notifier_rx,
            callbacks: HashMap::new(),
        }
    }

    pub fn bind_once(
        &mut self,
        binding_keys: Vec<BindingKey>,
        callback: Box<dyn FnOnce() + Send + 'static>,
    ) -> Option<u32> {
        self.bind(binding_keys, BindingCallback::Once(callback))
    }

    pub fn bind_multi(
        &mut self,
        binding_keys: Vec<BindingKey>,
        callback: Box<dyn FnMut()>,
    ) -> Option<u32> {
        self.bind(binding_keys, BindingCallback::Multi(callback))
    }

    fn bind(&mut self, binding_keys: Vec<BindingKey>, callback: BindingCallback) -> Option<u32> {
        let binding_info = BindingInfo::new(binding_keys, self.binding_notifier_tx.clone());
        let uid = binding_info.get_uid();
        if let Err(e) = self.binding_opt_tx.send(ListenerOpt::Bind(binding_info)) {
            println!("subscribe event failed, {}", e);
            return None;
        }
        self.callbacks.insert(uid, callback);
        Some(uid)
    }

    fn unbind(&mut self, binding_uid: u32) {
        if let Err(e) = self.binding_opt_tx.send(ListenerOpt::Unbind(binding_uid)) {
            println!("unbind:{} failed, {}", binding_uid, e);
        }
    }

    pub fn update(&mut self) {
        loop {
            match self.binding_notifier_rx.try_recv() {
                Ok(uid) => self.trigger_callback(uid),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    println!("disconnected");
                }
            }
        }
    }

    fn trigger_callback(&mut self, uid: u32) {
        match self.callbacks.remove(&uid) {
            Some(BindingCallback::Once(callback)) => {
                self.unbind(uid);
                callback();
            }
            Some(BindingCallback::Multi(mut callback)) => {
                callback();
                self.callbacks.insert(uid, BindingCallback::Multi(callback));
            }
            _ => {}
        }
    }
}

struct Listener {
    binding_key_mgr: BindingKeyMgr,
    binding_opt_rx: Receiver<ListenerOpt>,
}

impl Listener {
    fn new(binding_opt_rx: Receiver<ListenerOpt>) -> Self {
        Self {
            binding_key_mgr: BindingKeyMgr::new(),
            binding_opt_rx,
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
        while self.handle_event_opt() {
            match PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).0 {
                -1 => {
                    println!("get message failed");
                    return;
                }
                0 => std::thread::sleep(std::time::Duration::from_millis(1)),
                _ => (),
            };
        }
        let _ = UnhookWindowsHookEx(KEYBOARD_HOOK);
        let _ = UnhookWindowsHookEx(MOUSE_HOOK);
    }

    fn handle_event_opt(&mut self) -> bool {
        loop {
            match self.binding_opt_rx.try_recv() {
                Ok(ListenerOpt::Bind(binding_info)) => {
                    self.binding_key_mgr.bind(binding_info);
                }
                Ok(ListenerOpt::Unbind(uid)) => {
                    self.binding_key_mgr.unbind(uid);
                }
                Ok(ListenerOpt::StopListen) => {
                    return false;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    println!("try receive EventOpt failed, channel disconnected");
                    return false;
                }
            };
        }
        return true;
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
    if (ncode == HC_ACTION as i32) && (wparam.0 as u32 != WM_MOUSEMOVE) {
        let listener = LISTENER.get_mut().unwrap();
        if let Some(input_key) = InputKey::from(wparam, lparam) {
            listener.binding_key_mgr.on_input_key(input_key);
        } else {
            println!("parse event:{} failed", wparam.0);
        }
    }
    CallNextHookEx(hook, ncode, wparam, lparam)
}
