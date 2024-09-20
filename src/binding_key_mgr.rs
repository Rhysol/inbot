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

struct BindingInfo {
    binding_uid: u32,
    notifier: Sender<u32>,
    keys: Vec<BindingKey>,
    matching_index: usize, // index of `Self::keys`. when input key matched, the value of `matching_index` + 1
}
type BindingInfoMutRc = Rc<RefCell<BindingInfo>>;

pub struct BindingKeyMgr {
    binding_info: HashMap<u32, BindingInfoMutRc>,
    first_key: HashMap<KeyCode, HashMap<u32, BindingInfoMutRc>>,
    to_match_keys: HashMap<KeyCode, HashMap<u32, BindingInfoMutRc>>,
    holding_keys: HashSet<KeyCode>,
}

impl BindingKeyMgr {
    pub fn new() -> Self {
        Self {
            binding_info: HashMap::new(),
            first_key: HashMap::new(),
            to_match_keys: HashMap::new(),
            holding_keys: HashSet::new(),
        }
    }
}

impl BindingKeyMgr {
    pub fn on_input_key(&mut self, input_key: InputKey) {
        println!("{}", input_key);

        if input_key.opt == KeyOpt::Up {
            self.holding_keys.remove(&input_key.key);
        } else if input_key.opt == KeyOpt::Down {
            self.holding_keys.insert(input_key.key);
            // self.update_next_match_keys(input_key.key);
        }
    }
}
