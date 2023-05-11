use tokio::process::Command;

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
