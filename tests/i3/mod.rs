use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
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
    get_fakeroot_lib,
    get_faketime_lib,
    wait_for_file,
    LogOnDropChild,
    Test,
    FAKE_TIME,
};

// start nested x server displays at 10
static DISPLAY_ID: AtomicUsize = AtomicUsize::new(10);

const MAX_WAIT_TIME: Duration = Duration::new(5, 0);
pub const TEST_CONFIG_STR: &str = "@@@@ TEST CONFIGURATION FILE @@@@";
pub const SCREENSHOTS_DIR: &str = "screenshots";

fn create_i3_conf(socket_path: impl AsRef<Path>, config_file: impl AsRef<Path>) -> String {
    format!(
        r#"# i3 config file (v4)
# {}
font pango:IosevkaTerm Nerd Font 12

bindsym Escape exit

ipc-socket {socket}

bar {{
        font pango:IosevkaTerm Nerd Font 12
        padding 0 0 0 0
        position top
        tray_output none
        workspace_buttons yes
        status_command RUST_LOG=istat=trace {exe} --config {config}

        colors {{
            background #2e3440
            statusline #d8dee9
            separator  #4c566a

            # colorclass       border  bg      text
            focused_workspace  #81a1c1 #5e81ac #d8dee9
            active_workspace   #4c566a #434c5e #d8dee9
            inactive_workspace #3b4252 #2e3440 #7a869f
            urgent_workspace   #d24b59 #bf616a #2e3440
            binding_mode       #c67bb9 #b48ead #2e3440
        }}
}}
"#,
        TEST_CONFIG_STR,
        exe = get_current_exe().display(),
        config = config_file.as_ref().display(),
        socket = socket_path.as_ref().display(),
    )
}

pub struct X11Test<'a> {
    x_display: String,
    _x_server: LogOnDropChild,
    _i3: LogOnDropChild,
    i3_socket: PathBuf,
    screenshot_file: PathBuf,
    test: &'a Test,
}

impl<'a> X11Test<'a> {
    pub fn spawn(test: &'a Test) -> X11Test<'a> {
        // spawn nested X server
        let x_id = DISPLAY_ID.fetch_add(1, Ordering::SeqCst);
        let x_display = format!(":{}", x_id);
        let x_server = {
            let use_xephyr = env::var("XEPHYR").is_ok();
            let mut cmd = Command::new(if use_xephyr { "Xephyr" } else { "Xvfb" });
            cmd.arg(&x_display)
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
        };
        let x_server = LogOnDropChild::log_all(x_server);

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
                // config
                .arg("--config")
                .arg(&i3_config)
                // environment
                .envs(&test.env)
                // setup faketime & our fs mocks
                .env(
                    "LD_PRELOAD",
                    format!("{}:{}", get_faketime_lib(), get_fakeroot_lib()),
                )
                .env("FAKETIME", format!("@{}", FAKE_TIME))
                .env("FAKEROOT", &test.fakeroot)
                .env("FAKEROOT_DIRS", "1")
                // setup logs
                .env("RUST_LOG", "istat=trace")
                // spawn in nested X server
                .env_remove("I3SOCK")
                .env("DISPLAY", &x_display)
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

        let screenshots_dir = PathBuf::from(SCREENSHOTS_DIR);
        fs::create_dir_all(&screenshots_dir).unwrap();
        let screenshot_file = screenshots_dir.join(&test.name);
        X11Test {
            x_display,
            _x_server: x_server,
            _i3: i3,
            i3_socket,
            screenshot_file,
            test,
        }
    }

    pub fn exit(&self) {
        let output = self._cmd("i3-msg exit");
        assert_eq!(output.status.code(), Some(1));
    }

    fn _cmd(&self, cmd: impl AsRef<str>) -> Output {
        Command::new("sh")
            .env("I3SOCK", &self.i3_socket)
            .env("DISPLAY", &self.x_display)
            .env("LD_PRELOAD", get_fakeroot_lib())
            .env("FAKEROOT", &self.test.fakeroot)
            .env("FAKEROOT_DIRS", "1")
            .arg("-c")
            .arg(format!("{}", cmd.as_ref()))
            .output()
            .unwrap()
    }

    fn cmd(&self, cmd: impl AsRef<str>) -> Vec<u8> {
        let output = self._cmd(cmd.as_ref());
        if !output.status.success() {
            panic!(
                "{} failed, code={:?}\ncmd_stderr: {}",
                cmd.as_ref(),
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim()
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
        self.istat_ipc("get-bar")
    }

    pub fn istat_ipc(&self, ipc_cmd: impl AsRef<str>) -> Value {
        let ipc = get_exe("istat-ipc");
        serde_json::from_slice(&self.cmd(format!(
            "{ipc} {cmd}",
            ipc = ipc.display(),
            cmd = ipc_cmd.as_ref()
        )))
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

    pub fn screenshot(&self, bar_id: impl AsRef<str>) {
        let (x, y, w, h) = self.i3_get_bar_position(&bar_id);
        let file = {
            let p = self.screenshot_file.file_name().unwrap().to_str().unwrap();
            let name = format!("{}.png", p);
            self.screenshot_file.with_file_name(name)
        };

        self.cmd(format!(
            "{scrot} | {convert} > {file}",
            scrot = format!(
                "scrot --autoselect {x},{y},{w},{h} -",
                x = x,
                y = y,
                w = w,
                h = h
            ),
            convert = format!(
                "convert - -crop {w}x{h}+{x}+{y} -",
                w = w,
                h = h,
                x = w / 2,
                y = y,
            ),
            file = file.display()
        ));
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
            let x_test = crate::i3::X11Test::spawn(&test);
            $test_fn(&x_test);
            x_test.exit();
        }
    };
}

automod::dir!("tests/i3");
