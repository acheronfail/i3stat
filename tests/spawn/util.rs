use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;
use std::{env, fs};

use istat::config::AppConfig;
use istat::i3::{I3Button, I3ClickEvent, I3Modifier};
use istat::ipc::{encode_ipc_msg, IpcMessage, IpcReply, IpcResult, IPC_HEADER_LEN};
use serde_json::Value;
use timeout_readwrite::{TimeoutReadExt, TimeoutReader};

// util ------------------------------------------------------------------------

// mocked via libfaketime, see: https://github.com/wolfcw/libfaketime
pub const FAKE_TIME: &str = "1985-10-26 01:35:00";

const FAKE_TIME_LIB_PATHS: &[&str] = &[
    // Arch Linux
    "/usr/lib/faketime/libfaketime.so.1",
    // Debian/Ubuntu (used in CI)
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

/// Convenience struct for running assertions on and communicating with a running instance of the program
pub struct TestProgram {
    child: Child,
    socket: PathBuf,
    stdin: ChildStdin,
    stdout: BufReader<TimeoutReader<ChildStdout>>,
    stderr: ChildStderr,
}

impl TestProgram {
    /// Spawn the program, setting up it's own test directory
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
            // setup faketime
            .env("LD_PRELOAD", get_faketime_lib())
            .env("FAKETIME", format!("@{}", FAKE_TIME))
            // setup logs
            .env("RUST_LOG", "istat=trace")
            // socket
            .arg("--socket")
            .arg(&socket)
            // config
            .arg("--config")
            .arg(config_file)
            // stdio
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let stdin = child.stdin.take().unwrap();

        let stdout = child.stdout.take().unwrap();
        let stdout = stdout.with_timeout(Duration::from_secs(2));
        let stdout = BufReader::new(stdout);

        let stderr = child.stderr.take().unwrap();

        TestProgram {
            child,
            socket,
            stdin,
            stdout,
            stderr,
        }
    }

    /// Get the next line of STDOUT as a string - blocks
    pub fn next_line(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let mut line = String::new();
        let count = self.stdout.read_line(&mut line)?;
        Ok(if count == 0 {
            None
        } else {
            Some(line.trim().to_string())
        })
    }

    /// Send a raw click event
    pub fn click_raw(&mut self, click: I3ClickEvent) {
        self.stdin
            .write_all(&serde_json::to_vec(&click).unwrap())
            .unwrap();
        self.stdin.write_all(b"\n").unwrap();
    }

    /// Simple interface for sending click events
    pub fn click(&mut self, target: impl AsRef<str>, button: I3Button, modifiers: &[I3Modifier]) {
        self.click_raw(I3ClickEvent {
            instance: Some(target.as_ref().into()),
            button,
            modifiers: modifiers.to_vec(),
            ..Default::default()
        })
    }

    /// Send an IPC message to the running program
    pub fn send_ipc(&mut self, msg: IpcMessage) -> Value {
        let mut stream = UnixStream::connect(&self.socket).unwrap();
        stream.write_all(&encode_ipc_msg(msg).unwrap()).unwrap();

        let mut buf = vec![];
        stream.read_to_end(&mut buf).unwrap();
        serde_json::from_slice::<Value>(&buf[IPC_HEADER_LEN..]).unwrap()
    }

    /// Send a shutdown request via IPC
    pub fn send_shutdown(&mut self) {
        let reply = self.send_ipc(IpcMessage::Shutdown);
        let reply = serde_json::from_value::<IpcReply>(reply).unwrap();
        assert_eq!(reply, IpcReply::Result(IpcResult::Success(None)));
    }

    /// Gets the current config for the program
    pub fn get_config(&mut self) -> AppConfig {
        let reply = self.send_ipc(IpcMessage::GetConfig);
        let reply = serde_json::from_value::<IpcReply>(reply).unwrap();
        match reply {
            IpcReply::CustomResponse(value) => serde_json::from_value::<AppConfig>(value).unwrap(),
            _ => unreachable!(),
        }
    }

    /// Perform an assertion on the next line as JSON
    pub fn next_line_json(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        let next_line = self.next_line()?;
        Ok(match next_line {
            Some(line) => serde_json::from_str::<Value>(&line[..line.len() - 1])?,
            None => Value::Null,
        })
    }

    /// A message is emitted per item, so wait for all items to have emitted something
    pub fn wait_for_all_init(&mut self) {
        for _ in 0..self.get_config().items.len() - 1 {
            self.next_line_json().unwrap();
        }
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
