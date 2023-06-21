use std::ffi::{CStr, CString};
use std::os::unix::prelude::OsStrExt;
use std::path::PathBuf;
use std::{env, str};

use libc::{c_char, c_int};

const ENV_VAR: &str = "FAKE_ROOT";
const HOOK_TAG: &str = "@@@ HOOK @@@";

// TODO: configurable logging

fn get_fake_path(c_path: *const c_char) -> Option<CString> {
    // SAFETY: this is only called in the context of a hook, and strings passed here
    // are valid for the hook duration
    let c_str = unsafe { CStr::from_ptr(c_path) };

    // parse c string
    let path_str = match str::from_utf8(c_str.to_bytes()) {
        Ok(actual_path) => actual_path,
        Err(e) => {
            eprintln!("{}: str conv: {}", HOOK_TAG, e);
            return None;
        }
    };

    // get fake root
    // TODO: can we cache this call somehow rather than looking it up each time?
    let fake_root = match env::var(ENV_VAR) {
        Ok(path) => {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                eprintln!("{}: {} is not absolute!", HOOK_TAG, ENV_VAR);
                return None;
            }

            path
        }
        Err(e) => {
            eprintln!("{}: {}", HOOK_TAG, e);
            return None;
        }
    };

    // make path relative to our fake root
    // trim off leading `/` since `.join` will replace if it finds an absolute path
    let fake_path = fake_root.join(&path_str[1..]);
    if !fake_path.exists() {
        return None;
    }

    // we found a fake file, return a string representing its path
    eprintln!("{}: {} => {}", HOOK_TAG, path_str, fake_path.display());
    Some(CString::new(fake_path.as_os_str().as_bytes()).unwrap())
}

redhook::hook! {
    unsafe fn open64(path: *const c_char, flags: c_int, mode: c_int) -> c_int => my_open64 {
        let fake = get_fake_path(path);
        match fake {
            Some(c_str) => redhook::real!(open64)(c_str.as_ptr(), flags, mode),
            None => redhook::real!(open64)(path, flags, mode),
        }
    }
}
