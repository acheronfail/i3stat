use std::sync::{
    Arc,
    Mutex,
};

use sysinfo::{
    System,
    SystemExt,
};

pub type Ctx = Arc<Mutex<Context>>;

pub struct Context {
    _sys: System,
    // TODO: dbus
}

impl Context {
    pub fn new() -> Ctx {
        Arc::new(Mutex::new(Context {
            // TODO: only load what we need (depending on configuration, etc)
            _sys: System::new_all(),
        }))
    }
}
