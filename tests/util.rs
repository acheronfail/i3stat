use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs};

use serde_json::Value;

// util ------------------------------------------------------------------------

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

/// Find the location of the binary we're testing.
pub fn get_current_exe() -> PathBuf {
    env::current_exe()
        .expect("failed to find current_exe")
        .parent()
        .expect("failed to get parent dir")
        .join(format!("../istat{}", env::consts::EXE_SUFFIX))
        .canonicalize()
        .expect("failed to resolve path")
}

// test ------------------------------------------------------------------------

pub struct Test {
    pub env: HashMap<String, String>,
    pub bin_dir: PathBuf,
    pub socket_file: PathBuf,
    pub config_file: PathBuf,
}

impl Test {
    pub fn new(name: impl AsRef<str>, config: Value) -> Test {
        let name = name.as_ref();
        let dir = env::temp_dir().join(format!("istat-test-{}", name));
        let bin_dir = dir.join("bin");
        {
            if dir.exists() {
                fs::remove_dir_all(&dir).unwrap();
            }
            fs::create_dir_all(&bin_dir).unwrap();
        }

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
            env,
            bin_dir,
            config_file,
            socket_file,
        }
    }

    pub fn add_bin(&mut self, name: impl AsRef<str>, contents: impl AsRef<str>) {
        let mut file = File::create(self.bin_dir.join(name.as_ref())).unwrap();
        file.write_all(contents.as_ref().as_bytes()).unwrap();

        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata().unwrap().permissions();
        perms.set_mode(0o777);
        file.set_permissions(perms).unwrap();
    }
}

// spawn  ----------------------------------------------------------------------
