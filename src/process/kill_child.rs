// We can't simply do child.kill() because it wouldn't kill child's own
// child processes, such as ffmpeg conversion

use tokio::process::Child;

#[cfg(windows)]
pub async fn kill_child_process(child: &Child) -> anyhow::Result<()> {
    use crate::process::capture::run_command_to_end;
    use tokio::process::Command;
    use tracing::warn;

    if let Some(pid) = child.id() {
        let mut command = Command::new("taskkill");
        command.arg("/f").arg("/t").arg("/pid").arg(pid.to_string());

        let result = run_command_to_end(command).await?;

        if result.exit_code != Some(0) {
            warn!(
                "Failed to kill child process with pid = {pid}. Taskkill output:\n{:?}{:02X?}\n{:?}{:02X?}",
                String::from_utf8_lossy(&result.stdout), result.stdout,
                String::from_utf8_lossy(&result.stderr), result.stderr,
            );
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub async fn kill_child_process(child: &Child) -> anyhow::Result<()> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    if let Some(pid) = child.id() {
        signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM).unwrap();
    }
    Ok(())
}
