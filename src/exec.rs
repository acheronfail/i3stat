use tokio::process::Command;

pub async fn exec(cmd: impl AsRef<str>) {
    let cmd = cmd.as_ref();
    dbg!(cmd); // TODO: proper logging

    let child = Command::new("sh").arg("-c").arg(cmd).output();
    match child.await {
        Ok(output) => {
            if !output.status.success() {
                todo!(
                    "handle child proc exit status: {} exited with {}",
                    cmd,
                    output.status
                );
            }
        }
        Err(e) => todo!("handle child proc error: {}", e),
    }
}
