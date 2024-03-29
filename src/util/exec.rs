use std::process::Command;
use std::thread;

use crate::i3::I3Item;

/// Used when bar items need to run an external command. It won't block, and also
/// won't return any error: it shouldn't crash the app if the child process fails
/// in any way (just like i3 handles commands).
///
/// Unfortunately there's no `Command::try_wait` equivalent on `tokio::process::Command`,
/// so this spawns a separate thread for each command, in case the command blocks or waits.
pub fn exec(cmd: impl AsRef<str>, item: &I3Item) {
    let cmd = cmd.as_ref().to_owned();
    log::debug!("exec: command --> {} <--", &cmd);

    let env_map = item.as_env_map().unwrap();
    thread::spawn(move || {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .envs(env_map)
            .output();
        match output {
            Ok(output) => {
                if !output.status.success() {
                    log::warn!("exit: command --> {} <-- {}", cmd, output.status);
                }
            }
            Err(e) => log::error!("fail: command --> {} <-- {}", cmd, e),
        }
    });
}
