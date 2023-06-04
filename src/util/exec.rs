use tokio::process::Command;

/// Used when bar items need to run an external command. It won't block, and also
/// won't return any error: it shouldn't crash the app if the child process fails
/// in any way (just like i3 handles commands).
pub async fn exec(cmd: impl AsRef<str>) {
    let cmd = cmd.as_ref();
    log::debug!("exec: command --> {} <--", cmd);

    let child = Command::new("sh").arg("-c").arg(cmd).output();
    match child.await {
        Ok(output) => {
            if !output.status.success() {
                log::warn!("exit: command --> {} <-- {}", cmd, output.status);
            }
        }
        Err(e) => log::error!("fail: command --> {} <-- {}", cmd, e),
    }
}
