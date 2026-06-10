pub mod unix;
pub mod windows;

/// 데몬/서비스 명령어
#[derive(Debug, Clone)]
pub enum DaemonCommand {
    Install { use_task_scheduler: bool },
    Start,
    Stop,
    Status,
    Logs,
    Uninstall,
}

/// OS에 따라 적절한 데몬 백엔드 실행
pub fn execute(cmd: DaemonCommand) -> anyhow::Result<()> {
    if cfg!(windows) {
        windows::execute(cmd)
    } else {
        unix::execute(cmd)
    }
}
