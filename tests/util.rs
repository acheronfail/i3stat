use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use serde_json::Value;
use timeout_readwrite::TimeoutReadExt;

// faketime --------------------------------------------------------------------

// mocked via libfaketime, see: https://github.com/wolfcw/libfaketime
pub const FAKE_TIME: &str = "1985-10-26 01:35:00";

const FAKE_TIME_LIB_PATHS: &[&str] = &[
    // Arch Linux
    "/usr/lib/faketime/libfaketime.so.1",
    // Debian/Ubuntu (used in CI)
    "/usr/lib/x86_64-linux-gnu/faketime/libfaketime.so.1",
];

pub fn get_faketime_lib() -> &'static str {
    for path in FAKE_TIME_LIB_PATHS {
        if PathBuf::from(path).exists() {
            return *path;
        }
    }

    panic!("failed to find libfaketime.so.1");
}

// fakeroot --------------------------------------------------------------------

pub fn get_fakeroot_lib() -> String {
    get_exe("libfakeroot.so").display().to_string()
}

// misc ------------------------------------------------------------------------

fn get_exe_dir() -> PathBuf {
    env::current_exe()
        .expect("failed to find current_exe")
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn get_exe(name: impl AsRef<str>) -> PathBuf {
    get_exe_dir()
        .join(format!("{}{}", name.as_ref(), env::consts::EXE_SUFFIX))
        .canonicalize()
        .expect("failed to resolve path")
}

/// Find the location of the binary we're testing.
pub fn get_current_exe() -> PathBuf {
    get_exe("istat")
}

pub fn wait_for_file(path: impl AsRef<Path>, timeout: Duration) {
    let start = Instant::now();
    loop {
        thread::sleep(Duration::from_millis(100));
        if path.as_ref().exists() {
            break;
        }

        if start.elapsed() > timeout {
            panic!(
                "exceeded timeout while waiting for file={}",
                path.as_ref().display()
            );
        }
    }
}

// command ---------------------------------------------------------------------

enum Log {
    All,
    StdErrOnly,
}

pub struct LogOnDropChild {
    child: Child,
    log: Log,
}

impl LogOnDropChild {
    pub fn log_all(child: Child) -> LogOnDropChild {
        LogOnDropChild {
            child,
            log: Log::All,
        }
    }

    pub fn log_stderr(child: Child) -> LogOnDropChild {
        LogOnDropChild {
            child,
            log: Log::StdErrOnly,
        }
    }
}

impl Deref for LogOnDropChild {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl DerefMut for LogOnDropChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}

impl Drop for LogOnDropChild {
    fn drop(&mut self) {
        if env::var("DEBUG").is_ok() {
            macro_rules! get {
                ($std:expr) => {{
                    let mut r = $std
                        .take()
                        .unwrap()
                        .with_timeout(Duration::from_millis(100));
                    let mut s = String::new();
                    let _ = r.read_to_string(&mut s);
                    s
                }};
            }

            match self.log {
                Log::All => {
                    eprintln!("stdout: {}", get!(self.stdout).trim());
                    eprintln!("stderr: {}", get!(self.stderr).trim());
                }
                Log::StdErrOnly => {
                    eprintln!("stderr: {}", get!(self.stderr).trim());
                }
            }
        }

        let _ = self.kill();
    }
}

// test ------------------------------------------------------------------------

static UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Test {
    pub name: String,
    pub env: HashMap<String, String>,
    pub dir: PathBuf,
    pub bin_dir: PathBuf,
    pub fakeroot: PathBuf,
    pub istat_socket_file: PathBuf,
    pub istat_config_file: PathBuf,
}

impl Test {
    pub fn new(name: impl AsRef<str>, config: Value) -> Test {
        let name = name.as_ref();
        let dir = env::temp_dir().join(format!(
            "istat-test-{}.{}",
            name,
            UNIQUE_ID.fetch_add(1, Ordering::SeqCst)
        ));

        let bin_dir = dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let fake_root = dir.canonicalize().unwrap().join("fake_root");
        fs::create_dir_all(&fake_root).unwrap();

        let socket_file = dir.join("socket");
        let config_file = dir.join("config.json");
        fs::write(&config_file, config.to_string()).unwrap();

        let mut env = HashMap::new();
        env.insert(
            "PATH".into(),
            format!(
                "{}:{}",
                bin_dir.to_str().unwrap(),
                env::var("PATH").unwrap()
            ),
        );

        Test {
            name: name.into(),
            dir,
            env,
            bin_dir,
            fakeroot: fake_root,
            istat_config_file: config_file,
            istat_socket_file: socket_file,
        }
    }

    pub fn add_bin(&self, name: impl AsRef<str>, contents: impl AsRef<str>) {
        let mut file = File::create(self.bin_dir.join(name.as_ref())).unwrap();
        file.write_all(contents.as_ref().as_bytes()).unwrap();

        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata().unwrap().permissions();
        perms.set_mode(0o777);
        file.set_permissions(perms).unwrap();
    }

    pub fn add_fake_file(&self, name: impl AsRef<str>, contents: impl AsRef<str>) {
        let name = name.as_ref();
        let name = if name.starts_with("/") {
            &name[1..]
        } else {
            name
        };

        let path = self.fakeroot.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();

        let mut file = File::create(&path).unwrap();
        file.write_all(contents.as_ref().as_bytes()).unwrap();
    }
}

impl Drop for Test {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.dir).unwrap();
    }
}

// serde_json ------------------------------------------------------------------

/// Check if `needle` is a subset or match of `haystack`
pub fn json_contains_inner(haystack: &Value, needle: &Value) {
    use Value::*;

    match (haystack, needle) {
        (Object(haystack), Object(needle)) => {
            for (k, v) in needle {
                match haystack.get(k) {
                    Some(value) => json_contains_inner(value, v),
                    None => panic!("object did not contain key: {}", k),
                }
            }
        }
        (Array(haystack), Array(needle)) => {
            assert_eq!(
                haystack.len(),
                needle.len(),
                "arrays are of different lengths"
            );

            for idx in 0..haystack.len() {
                json_contains_inner(&haystack[idx], &needle[idx]);
            }
        }
        (String(haystack), String(needle)) => assert_eq!(haystack, needle),
        (Number(haystack), Number(needle)) => assert_eq!(haystack, needle),
        (Bool(haystack), Bool(needle)) => assert_eq!(haystack, needle),
        (Null, Null) => {}
        _ => panic!("both values must be the same type"),
    }
}

/// Return a path to an object containing a specific key and value
pub fn find_object_containing<'a>(
    root: &'a Value,
    key: &'static str,
    value: &'a Value,
) -> Vec<&'a Value> {
    macro_rules! find {
        ($value:expr) => {
            let path = find_object_containing($value, key, value);
            if !path.is_empty() {
                let mut result = vec![root];
                result.extend(path);
                return result;
            }
        };
    }

    match root {
        Value::Array(arr) => {
            for element in arr {
                find!(element);
            }
        }
        Value::Object(map) => {
            for (k, v) in map {
                if key == k && value == v {
                    return vec![root];
                }

                find!(v);
            }
        }
        _ => {}
    }

    vec![]
}
