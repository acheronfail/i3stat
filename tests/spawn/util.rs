use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::time::Duration;
use std::{env, fs};

use istat::ipc::{encode_ipc_msg, IpcMessage, IpcReply, IpcResult, IPC_HEADER_LEN};
use serde_json::Value;
use timeout_readwrite::{TimeoutReadExt, TimeoutReader};

// util ------------------------------------------------------------------------

// mocked via libfaketime, see: https://github.com/wolfcw/libfaketime
pub const FAKE_TIME: &str = "1985-10-26 01:35:00";

const FAKE_TIME_LIB_PATHS: &[&str] = &[
    // Arch Linux
    "/usr/lib/faketime/libfaketime.so.1",
    // Debian
    "/usr/lib/x86_64-linux-gnu/faketime/libfaketime.so.1",
];

fn get_faketime_lib() -> &'static str {
    for path in FAKE_TIME_LIB_PATHS {
        if PathBuf::from(path).exists() {
            return *path;
        }
    }

    panic!("failed to find libfaketime.so.1");
}

/// Find the location of the binary we're testing.
fn get_current_exe() -> PathBuf {
    env::current_exe()
        .expect("failed to find current_exe")
        .parent()
        .expect("failed to get parent dir")
        .join(format!("../istat{}", env::consts::EXE_SUFFIX))
        .canonicalize()
        .expect("failed to resolve path")
}

// spawn  ----------------------------------------------------------------------

pub struct TestProgram {
    child: Child,
    socket: PathBuf,
    stdout: BufReader<TimeoutReader<ChildStdout>>,
    stderr: ChildStderr,
}

impl TestProgram {
    pub fn run(name: impl AsRef<str>, config: Value) -> TestProgram {
        let name = name.as_ref();
        let test_dir = env::temp_dir().join(format!("istat-test-{}", name));
        {
            if test_dir.exists() {
                fs::remove_dir_all(&test_dir).unwrap();
            }
            fs::create_dir_all(&test_dir).unwrap();
        }

        let socket = test_dir.join("socket");
        let config_file = test_dir.join("config.json");
        fs::write(&config_file, config.to_string()).unwrap();

        let mut child = Command::new(get_current_exe())
            .env("LD_PRELOAD", get_faketime_lib())
            .env("FAKETIME", format!("@{}", FAKE_TIME))
            .arg("--socket")
            .arg(&socket)
            .arg("--config")
            .arg(config_file)
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let stdout = child.stdout.take().unwrap();
        let stdout = stdout.with_timeout(Duration::from_secs(3));
        let stdout = BufReader::new(stdout);

        let stderr = child.stderr.take().unwrap();

        TestProgram {
            child,
            socket,
            stdout,
            stderr,
        }
    }

    pub fn next_line(&mut self) -> Option<String> {
        let mut line = String::new();
        let count = self.stdout.read_line(&mut line).unwrap();
        if count == 0 {
            None
        } else {
            Some(line.trim().to_string())
        }
    }

    pub fn shutdown(&mut self) {
        let mut stream = UnixStream::connect(&self.socket).unwrap();

        let msg = encode_ipc_msg(IpcMessage::Shutdown).unwrap();
        stream.write_all(&msg).unwrap();

        let mut buf = vec![];
        stream.read_to_end(&mut buf).unwrap();
        let resp = serde_json::from_slice::<IpcReply>(&buf[IPC_HEADER_LEN..]).unwrap();
        assert_eq!(resp, IpcReply::Result(IpcResult::Success(None)));
    }

    pub fn assert_next_line(&mut self, expected: Option<&str>) {
        assert_eq!(self.next_line().as_deref(), expected);
    }

    pub fn assert_next_line_json(&mut self, expected: Value) {
        let next_line = self.next_line();
        match next_line {
            Some(line) => {
                assert_eq!(
                    serde_json::from_str::<Value>(&line[..line.len() - 1]).unwrap(),
                    expected
                );
            }
            None => assert_eq!(Value::Null, expected),
        };
    }

    pub fn assert_i3_header(&mut self) {
        self.assert_next_line(Some(r#"{"version":1,"click_events":true}"#));
        self.assert_next_line(Some(r#"["#));
    }
}

impl Drop for TestProgram {
    fn drop(&mut self) {
        // terminate child
        let _ = self.child.kill();

        // get any stderr and log it
        {
            let mut stderr = String::new();
            self.stderr.read_to_string(&mut stderr).unwrap();
            eprintln!("stderr: {:?}", stderr.trim());
        }
    }
}

// macros ----------------------------------------------------------------------

#[macro_export]
macro_rules! spawn_test {
    ($name:ident, $config:expr, $test_fn:expr) => {
        #[test]
        fn $name() {
            $test_fn(crate::util::TestProgram::run(stringify!($name), $config));
        }
    };
}
