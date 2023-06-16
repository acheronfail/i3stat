use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

use istat::config::AppConfig;
use istat::i3::{I3Button, I3ClickEvent, I3Modifier};
use istat::ipc::protocol::{encode_ipc_msg, IpcMessage, IpcReply, IpcResult, IPC_HEADER_LEN};
use serde_json::Value;
use timeout_readwrite::{TimeoutReadExt, TimeoutReader};

use crate::util::{get_current_exe, get_faketime_lib, LogOnDropChild, Test, FAKE_TIME};

/// Convenience struct for running assertions on and communicating with a running instance of the program
pub struct SpawnedProgram {
    #[allow(unused)]
    child: LogOnDropChild,
    socket: PathBuf,
    stdin: ChildStdin,
    stdout: BufReader<TimeoutReader<ChildStdout>>,
}

impl SpawnedProgram {
    /// Spawn the program, setting up it's own test directory
    pub fn spawn(test: &Test) -> SpawnedProgram {
        let mut child = LogOnDropChild::log_stderr(
            Command::new(get_current_exe())
                .envs(&test.env)
                // setup faketime
                .env("LD_PRELOAD", get_faketime_lib())
                .env("FAKETIME", format!("@{}", FAKE_TIME))
                // setup logs
                .env("RUST_LOG", "istat=trace")
                // socket
                .arg("--socket")
                .arg(&test.istat_socket_file)
                // config
                .arg("--config")
                .arg(&test.istat_config_file)
                // stdio
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        );

        let stdin = child.stdin.take().unwrap();

        let stdout = child.stdout.take().unwrap();
        let stdout = stdout.with_timeout(Duration::from_secs(2));
        let stdout = BufReader::new(stdout);

        let mut test = SpawnedProgram {
            child,
            socket: test.istat_socket_file.clone(),
            stdin,
            stdout,
        };

        // assert header
        assert_eq!(
            test.next_line().unwrap().as_deref(),
            Some(r#"{"version":1,"click_events":true}"#)
        );
        assert_eq!(test.next_line().unwrap().as_deref(), Some(r#"["#));

        // wait for all items to start up
        test.wait_for_all_init();

        test
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

    /// Send a raw click event via STDIN
    pub fn click_raw(&mut self, click: I3ClickEvent) {
        self.stdin
            .write_all(&serde_json::to_vec(&click).unwrap())
            .unwrap();
        self.stdin.write_all(b"\n").unwrap();
    }

    /// Simple interface for sending click events via STDIN
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

    /// Gets the current config for the program via IPC
    pub fn get_config(&mut self) -> AppConfig {
        let reply = self.send_ipc(IpcMessage::GetConfig);
        let reply = serde_json::from_value::<IpcReply>(reply).unwrap();
        match reply {
            IpcReply::Value(value) => serde_json::from_value::<AppConfig>(value).unwrap(),
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
    fn wait_for_all_init(&mut self) {
        for _ in 0..self.get_config().items.len().saturating_sub(1) {
            self.next_line_json().unwrap();
        }
    }
}

macro_rules! spawn_test {
    ($name:ident, $config:expr, $test_fn:expr) => {
        spawn_test!($name, $config, |x| x, $test_fn);
    };

    ($name:ident, $config:expr, $setup_fn:expr, $test_fn:expr) => {
        #[test]
        fn $name() {
            let mut test = crate::util::Test::new(stringify!($name), $config);
            $setup_fn(&mut test);
            let istat = crate::spawn::SpawnedProgram::spawn(&test);
            $test_fn(istat);
        }
    };
}

automod::dir!("tests/spawn");
