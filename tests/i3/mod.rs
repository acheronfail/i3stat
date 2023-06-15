use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use std::{env, fs};

use serde_json::Value;

use self::util::x_click;
use crate::i3::util::MouseButton;
use crate::util::{
    find_object_containing,
    get_current_exe,
    get_exe,
    get_faketime_lib,
    wait_for_file,
    LogOnDropChild,
    Test,
    FAKE_TIME,
};

// start nested x server displays at 10
static DISPLAY_ID: AtomicUsize = AtomicUsize::new(10);

const MAX_WAIT_TIME: Duration = Duration::new(2, 0);
pub const TEST_CONFIG_STR: &str = "@@@@ TEST CONFIGURATION FILE @@@@";

fn create_i3_conf(socket_path: impl AsRef<Path>, config_file: impl AsRef<Path>) -> String {
    format!(
        r#"# i3 config file (v4)
# {}
font pango:IosevkaTerm Nerd Font 12

bindsym Escape exit

ipc-socket {socket}

bar {{
        font pango:IosevkaTerm Nerd Font 12
        position top
        tray_output none
        status_command RUST_LOG=istat=trace {exe} --config {config}
}}
"#,
        TEST_CONFIG_STR,
        exe = get_current_exe().display(),
        config = config_file.as_ref().display(),
        socket = socket_path.as_ref().display(),
    )
}

pub struct X11Test {
    x_display: String,
    _x_server: LogOnDropChild,
    _i3: LogOnDropChild,
    i3_socket: PathBuf,
}

impl X11Test {
    pub fn new(test: &Test) -> X11Test {
        // spawn nested X server
        let x_id = DISPLAY_ID.fetch_add(1, Ordering::SeqCst);
        let x_display = format!(":{}", x_id);
        let x_server = LogOnDropChild::log_all({
            let use_xephyr = env::var("XEPHYR").is_ok();
            let mut cmd = Command::new(if use_xephyr { "Xephyr" } else { "Xvfb" });
            let cmd = cmd
                // X display
                .arg(&x_display)
                .arg("-ac") // disable access control restrictions
                .arg("-br") // create root window with black background
                .arg("-reset") // reset after last client exists
                .arg("-terminate"); // terminate at server reset

            // screen size - different formats for Xephyr vs Xvfb
            let cmd = if use_xephyr {
                cmd.arg("-screen").arg("1900x200")
            } else {
                cmd.arg("-screen").arg("0").arg("1900x200x24")
            };

            // stdio
            cmd.stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap()
        });

        // create i3 config file
        let i3_config = test.dir.join("i3.conf");
        let i3_socket = test.dir.join("i3.sock");
        let istat_socket = test.dir.join("i3.sock.istat");
        fs::write(
            &i3_config,
            create_i3_conf(&i3_socket, &test.istat_config_file),
        )
        .unwrap();

        // wait for X server to start
        wait_for_file(
            PathBuf::from(format!("/tmp/.X11-unix/X{}", x_id)),
            MAX_WAIT_TIME,
        );

        // spawn i3 in newly created X server
        let i3 = LogOnDropChild::log_all(
            Command::new("i3")
                .envs(&test.env)
                // setup faketime
                .env("LD_PRELOAD", get_faketime_lib())
                .env("FAKETIME", format!("@{}", FAKE_TIME))
                // setup logs
                .env("RUST_LOG", "istat=trace")
                // spawn in nested X server
                .env_remove("I3SOCK")
                .env("DISPLAY", &x_display)
                // config
                .arg("--config")
                .arg(&i3_config)
                // stdio
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        );

        // wait for i3's socket to appear
        wait_for_file(&i3_socket, MAX_WAIT_TIME);

        // wait for istat's socket to appear
        wait_for_file(&istat_socket, MAX_WAIT_TIME);

        X11Test {
            x_display,
            _x_server: x_server,
            _i3: i3,
            i3_socket,
        }
    }

    fn cmd(&self, cmd: impl AsRef<str>) -> Vec<u8> {
        let output = Command::new("sh")
            .env("I3SOCK", &self.i3_socket)
            .env("DISPLAY", &self.x_display)
            .arg("-c")
            .arg(format!("{}", cmd.as_ref()))
            .output()
            .unwrap();

        if !output.status.success() {
            panic!(
                "{} failed, code={:?}\nstderr: {}",
                cmd.as_ref(),
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        output.stdout
    }

    pub fn i3_get_tree(&self) -> Value {
        serde_json::from_slice(&self.cmd("i3-msg -t get_tree")).unwrap()
    }

    pub fn i3_get_bar_position(&self, bar_id: impl AsRef<str>) -> (i16, i16, u16, u16) {
        let tree = self.i3_get_tree();
        let v = Value::String(bar_id.as_ref().into());
        // this finds object containing `instance: $bar_id` in i3's tree, which is the
        // node's "window_properties" object
        let path = find_object_containing(&tree, "instance", &v);
        assert!(!path.is_empty(), "failed to find i3 bar node in tree");
        // get parent node
        let node = path[path.len() - 2];
        // get rect
        let r = node.get("rect").unwrap();
        return (
            r.get("x").unwrap().as_i64().unwrap() as _,
            r.get("y").unwrap().as_i64().unwrap() as _,
            r.get("width").unwrap().as_i64().unwrap() as _,
            r.get("height").unwrap().as_i64().unwrap() as _,
        );
    }

    pub fn i3_get_config(&self) -> String {
        String::from_utf8(self.cmd("i3-msg -t get_config")).unwrap()
    }

    pub fn istat_get_bar(&self) -> Value {
        serde_json::from_slice(&self.cmd(format!("{} get-bar", get_exe("istat-ipc").display())))
            .unwrap()
    }

    pub fn i3_get_bars(&self) -> Vec<String> {
        serde_json::from_slice(&self.cmd("i3-msg -t get_bar_config")).unwrap()
    }

    pub fn i3_get_bar(&self, bar_id: impl AsRef<str>) -> Value {
        serde_json::from_slice(&self.cmd(format!("i3-msg -t get_bar_config {}", bar_id.as_ref())))
            .unwrap()
    }

    pub fn click(&self, button: MouseButton, x: i16, y: i16) {
        x_click(&self.x_display, button, x, y)
    }
}

macro_rules! x_test {
    ($name:ident, $config:expr, $test_fn:expr) => {
        x_test!($name, $config, |x| x, $test_fn);
    };

    ($name:ident, $config:expr, $setup_fn:expr, $test_fn:expr) => {
        #[test]
        fn $name() {
            let mut test = crate::util::Test::new(stringify!($name), $config);
            $setup_fn(&mut test);
            let x_test = crate::i3::X11Test::new(&test);
            $test_fn(x_test);
        }
    };
}

automod::dir!("tests/i3");
