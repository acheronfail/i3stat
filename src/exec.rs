use tokio::process::Command;

pub async fn exec(cmd: impl AsRef<str>) {
    let cmd = cmd.as_ref();
    let parts = cmd.split_whitespace().collect::<Vec<_>>();
    let mut child = &mut Command::new(parts[0]);
    for part in parts.iter().skip(1) {
        child = child.arg(part);
    }

    match child.output().await {
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
