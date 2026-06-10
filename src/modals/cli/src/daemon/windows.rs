use std::process::Command;

use anyhow::{Context, Result};
use tracing::info;

use super::DaemonCommand;

const SERVICE_NAME: &str = "FilePipeline";
const TASK_NAME: &str = "\\FilePipeline\\Watch";

pub fn execute(cmd: DaemonCommand) -> Result<()> {
    match cmd {
        DaemonCommand::Install { use_task_scheduler } => {
            if use_task_scheduler {
                install_task_scheduler()
            } else {
                install_windows_service()
            }
        }
        DaemonCommand::Start => start_service(),
        DaemonCommand::Stop => stop_service(),
        DaemonCommand::Status => show_status(),
        DaemonCommand::Logs => tail_logs(),
        DaemonCommand::Uninstall => uninstall(),
    }
}

fn install_windows_service() -> Result<()> {
    let exe = std::env::current_exe()?;
    let exe_path = exe.to_string_lossy();

    let output = Command::new("sc.exe")
        .args([
            "create",
            SERVICE_NAME,
            &format!("binPath= \"{}\" watch", exe_path),
            "start=",
            "auto",
            "DisplayName=",
            "File Processing Pipeline",
        ])
        .output()
        .context("sc.exe 실행 실패 (관리자 권한 필요)")?;

    if output.status.success() {
        info!("Windows Service '{}' 등록 완료", SERVICE_NAME);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("서비스 등록 실패: {}", stderr);
    }

    Ok(())
}

fn install_task_scheduler() -> Result<()> {
    let exe = std::env::current_exe()?;
    let exe_path = exe.to_string_lossy();

    let output = Command::new("schtasks.exe")
        .args([
            "/Create",
            "/TN", TASK_NAME,
            "/TR", &format!("\"{}\" watch", exe_path),
            "/SC", "ONLOGON",
            "/RL", "LIMITED",
            "/F",
        ])
        .output()
        .context("schtasks.exe 실행 실패")?;

    if output.status.success() {
        info!("Task Scheduler '{}' 등록 완료", TASK_NAME);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("작업 등록 실패: {}", stderr);
    }

    Ok(())
}

fn start_service() -> Result<()> {
    // sc.exe로 시도, 실패 시 schtasks 시도
    let output = Command::new("sc.exe")
        .args(["start", SERVICE_NAME])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            info!("서비스 시작됨");
            Ok(())
        }
        _ => {
            let output = Command::new("schtasks.exe")
                .args(["/Run", "/TN", TASK_NAME])
                .output()
                .context("서비스/작업 시작 실패")?;

            if output.status.success() {
                info!("작업 스케줄러에서 시작됨");
                Ok(())
            } else {
                anyhow::bail!("서비스/작업 시작 실패")
            }
        }
    }
}

fn stop_service() -> Result<()> {
    let _ = Command::new("sc.exe")
        .args(["stop", SERVICE_NAME])
        .output();
    let _ = Command::new("schtasks.exe")
        .args(["/End", "/TN", TASK_NAME])
        .output();
    info!("서비스 중지 요청");
    Ok(())
}

fn show_status() -> Result<()> {
    let output = Command::new("sc.exe")
        .args(["query", SERVICE_NAME])
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            println!("{}", String::from_utf8_lossy(&o.stdout));
            return Ok(());
        }
    }

    let output = Command::new("schtasks.exe")
        .args(["/Query", "/TN", TASK_NAME, "/V", "/FO", "LIST"])
        .output()
        .context("상태 조회 실패")?;

    println!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

fn tail_logs() -> Result<()> {
    let log_path = file_pipeline_shared::platform::default_base_dir()
        .join("logs")
        .join("pipeline.log");

    println!("로그 파일: {}", log_path.display());

    let output = Command::new("powershell.exe")
        .args([
            "-Command",
            &format!("Get-Content '{}' -Tail 50 -Wait", log_path.display()),
        ])
        .status()
        .context("로그 tail 실패")?;

    if !output.success() {
        anyhow::bail!("로그 조회 실패");
    }

    Ok(())
}

fn uninstall() -> Result<()> {
    let _ = Command::new("sc.exe")
        .args(["delete", SERVICE_NAME])
        .output();
    let _ = Command::new("schtasks.exe")
        .args(["/Delete", "/TN", TASK_NAME, "/F"])
        .output();
    info!("서비스/작업 제거 완료");
    Ok(())
}
