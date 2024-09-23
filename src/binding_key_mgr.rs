use crate::virtual_key::*;
use std::sync::mpsc::Sender;
pub struct BindingKey {
    pub key: KeyCode,
    pub modifer_keys: Vec<KeyCode>,
}
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct BindingInfo {
    binding_uid: u32,
    notifier: Sender<u32>,
    keys: Vec<BindingKey>,
    matching_index: usize, // index of `Self::keys`. when input key matched, the value of `matching_index` + 1
}
type BindingInfoMutRc = Rc<RefCell<BindingInfo>>;

enum MatchKeyResult {
    Failed,            // 不匹配
    Matching(KeyCode), // 匹配成功，后续还有按键需要继续匹配
    Success,           // 完全匹配
}

impl BindingInfo {
    pub fn new(binding_keys: Vec<BindingKey>, notifier: Sender<u32>) -> Self {
        static LAST_ALLOCATED_SUBSCRIPTION_UID: AtomicU32 = AtomicU32::new(0);
        let uid = LAST_ALLOCATED_SUBSCRIPTION_UID.fetch_add(1, Ordering::SeqCst) + 1;
        Self {
            binding_uid: uid,
            notifier,
            keys: binding_keys,
            matching_index: 0,
        }
    }

    fn try_match(&mut self, input_key: KeyCode, holding_keys: &HashSet<KeyCode>) -> MatchKeyResult {
        if !self.if_key_matched(input_key, holding_keys) {
            self.matching_index = 0;
            return MatchKeyResult::Failed;
        }
        self.matching_index += 1;
        if let Some(to_match_key) = self.keys.get(self.matching_index) {
            return MatchKeyResult::Matching(to_match_key.key);
        }
        self.matching_index = 0;
        let _ = self.notifier.send(self.binding_uid);
        return MatchKeyResult::Success;
    }

    fn if_key_matched(&self, key: KeyCode, holding_keys: &HashSet<KeyCode>) -> bool {
        let matching_key = self.keys.get(self.matching_index);
        if matching_key.is_none() {
            return false;
        }
        let matching_key = matching_key.unwrap();
        if matching_key.key != key {
            return false;
        }
        for must_holding_key in &matching_key.modifer_keys {
            if !holding_keys.contains(&must_holding_key) {
                return false;
            }
        }
        if matching_key.modifer_keys.len() + 1 != holding_keys.len() {
            return false;
        }
        return true;
    }

    pub fn get_first_key(&self) -> Option<KeyCode> {
        if self.keys.is_empty() {
            return None;
        }
        return Some(self.keys.first().unwrap().key);
    }

    pub fn get_uid(&self) -> u32 {
        self.binding_uid
    }
}

pub struct BindingKeyMgr {
    bindings_info: HashMap<u32, BindingInfoMutRc>,
    first_key_to_match: HashMap<KeyCode, HashMap<u32, BindingInfoMutRc>>,
    to_match_keys: HashMap<KeyCode, HashMap<u32, BindingInfoMutRc>>,
    holding_keys: HashSet<KeyCode>,
}

impl BindingKeyMgr {
    pub fn new() -> Self {
        Self {
            bindings_info: HashMap::new(),
            first_key_to_match: HashMap::new(),
            to_match_keys: HashMap::new(),
            holding_keys: HashSet::new(),
        }
    }
}

impl BindingKeyMgr {
    pub fn bind(&mut self, binding_info: BindingInfo) {
        let uid = binding_info.binding_uid;
        if self.bindings_info.contains_key(&uid) {
            println!("subscription uid:{} already exist", uid);
            return;
        }
        let first_key = binding_info.get_first_key();
        if first_key.is_none() {
            return;
        }
        let first_key = first_key.unwrap();
        let bindings_of_keys = self.first_key_to_match.entry(first_key).or_default();
        bindings_of_keys.insert(
            binding_info.binding_uid,
            Rc::new(RefCell::new(binding_info)),
        );
    }

    pub fn unbind(&mut self, uid: u32) {
        let binding_info = self.bindings_info.remove(&uid);
        if binding_info.is_none() {
            return;
        }
        let binding_info = binding_info.unwrap();
        let binding_info = binding_info.borrow();
        let first_key = binding_info.get_first_key();
        if first_key.is_none() {
            return;
        }
        let first_key = first_key.unwrap();
        if let Some(bindings_of_key) = self.first_key_to_match.get_mut(&first_key) {
            bindings_of_key.remove(&binding_info.binding_uid);
        }
        for (_, bindings_of_key) in &mut self.to_match_keys {
            bindings_of_key.remove(&binding_info.binding_uid);
        }
    }

    pub fn on_input_key(&mut self, input_key: InputKey) {
        if input_key.opt == KeyOpt::Up {
            self.holding_keys.remove(&input_key.key);
        } else if input_key.opt == KeyOpt::Down {
            self.holding_keys.insert(input_key.key);
            self.update_next_match_keys(input_key.key);
        }
    }

    /// self.first_key
    ///     记录了首次可以匹配的按键
    /// self.to_match_keys
    ///     记录了下次可以匹配的按键，至少成功匹配过一个按键后该字段才会有值
    ///     有两种情况该内容会被更新:
    ///     * self.first_key匹配成功, 还有后续要匹配的按键, 例如组合按键：(Ctrl + A, Ctrl + B)
    ///     * self.to_match_keys匹配成功，后续还有按键要继续匹配， 例如组合按键：(Ctrl + A, Ctrl + B, Ctrl + C)
    ///
    /// 匹配规则
    /// 根据self.to_match_keys是否为空来判断目前是否成功匹配过任何按键
    ///     * 是，使用self.to_match_keys中的内容进行匹配
    ///     * 否，使用self.first_key中的内容进行匹配
    /// 当按键匹配成功后, 判断是否为组合按键
    ///     * 是，把后一个要匹配的按键更新到self.to_match_keys
    ///     * 否，匹配成功, 清空self.to_match_keys, 重新开始匹配
    /// 当匹配失败后, 清空self.to_match_keys, 重新开始匹配
    fn update_next_match_keys(&mut self, input_key: KeyCode) {
        let mut bindings_of_key = None;
        if self.to_match_keys.is_empty() {
            bindings_of_key = self.first_key_to_match.get(&input_key);
        } else {
            bindings_of_key = self.to_match_keys.get(&input_key);
        }
        // 按键没有匹配到任何绑定，重新开始匹配
        if bindings_of_key.is_none() {
            self.to_match_keys.drain();
            return;
        }
        let bindings_of_key = bindings_of_key.unwrap();
        let mut if_match_success = false;
        let mut next_match_keys: HashMap<KeyCode, HashMap<u32, BindingInfoMutRc>> = HashMap::new();
        for (uid, binding_info_rc) in bindings_of_key {
            let mut binding_info = binding_info_rc.borrow_mut();
            match binding_info.try_match(input_key, &self.holding_keys) {
                MatchKeyResult::Matching(next_key) => {
                    let next_match = next_match_keys.entry(next_key).or_default();
                    next_match.insert(*uid, binding_info_rc.clone());
                }
                MatchKeyResult::Success => {
                    if_match_success = true;
                }
                _ => {}
            }
        }
        // 有成功匹配的绑定，重新开始匹配
        if if_match_success {
            self.to_match_keys.drain();
            return;
        }
        self.to_match_keys = next_match_keys;
    }
}
