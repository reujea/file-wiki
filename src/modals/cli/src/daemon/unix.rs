use std::process::Command;

use anyhow::{Context, Result};
use tracing::info;

use super::DaemonCommand;

pub fn execute(cmd: DaemonCommand) -> Result<()> {
    match cmd {
        DaemonCommand::Install { use_task_scheduler: _ } => install(),
        DaemonCommand::Start => start(),
        DaemonCommand::Stop => stop(),
        DaemonCommand::Status => status(),
        DaemonCommand::Logs => logs(),
        DaemonCommand::Uninstall => uninstall(),
    }
}

fn service_dir() -> std::path::PathBuf {
    if cfg!(target_os = "macos") {
        dirs::home_dir()
            .unwrap_or_default()
            .join("Library")
            .join("LaunchAgents")
    } else {
        // Linux systemd user unit
        dirs::config_dir()
            .unwrap_or_default()
            .join("systemd")
            .join("user")
    }
}

fn install() -> Result<()> {
    let exe = std::env::current_exe()?;
    let exe_path = exe.to_string_lossy();

    if cfg!(target_os = "macos") {
        install_launchd(&exe_path)
    } else {
        install_systemd(&exe_path)
    }
}

fn install_launchd(exe_path: &str) -> Result<()> {
    let plist_dir = service_dir();
    std::fs::create_dir_all(&plist_dir)?;

    let plist_path = plist_dir.join("com.filepipeline.watch.plist");
    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.filepipeline.watch</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>watch</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/filepipeline.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/filepipeline.stderr.log</string>
</dict>
</plist>"#,
        exe_path
    );

    std::fs::write(&plist_path, plist_content)?;
    info!("launchd plist 생성: {:?}", plist_path);
    Ok(())
}

fn install_systemd(exe_path: &str) -> Result<()> {
    let unit_dir = service_dir();
    std::fs::create_dir_all(&unit_dir)?;

    let unit_path = unit_dir.join("file-pipeline.service");
    let unit_content = format!(
        r#"[Unit]
Description=File Processing Pipeline
After=network.target

[Service]
Type=simple
ExecStart={} watch
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
"#,
        exe_path
    );

    std::fs::write(&unit_path, unit_content)?;

    Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output()
        .context("systemctl daemon-reload 실패")?;

    Command::new("systemctl")
        .args(["--user", "enable", "file-pipeline.service"])
        .output()
        .context("systemctl enable 실패")?;

    info!("systemd unit 생성 및 활성화: {:?}", unit_path);
    Ok(())
}

fn start() -> Result<()> {
    if cfg!(target_os = "macos") {
        Command::new("launchctl")
            .args(["load", "-w"])
            .arg(
                service_dir()
                    .join("com.filepipeline.watch.plist")
                    .to_string_lossy()
                    .as_ref(),
            )
            .output()
            .context("launchctl load 실패")?;
    } else {
        Command::new("systemctl")
            .args(["--user", "start", "file-pipeline.service"])
            .output()
            .context("systemctl start 실패")?;
    }
    info!("서비스 시작됨");
    Ok(())
}

fn stop() -> Result<()> {
    if cfg!(target_os = "macos") {
        Command::new("launchctl")
            .args(["unload"])
            .arg(
                service_dir()
                    .join("com.filepipeline.watch.plist")
                    .to_string_lossy()
                    .as_ref(),
            )
            .output()
            .context("launchctl unload 실패")?;
    } else {
        Command::new("systemctl")
            .args(["--user", "stop", "file-pipeline.service"])
            .output()
            .context("systemctl stop 실패")?;
    }
    info!("서비스 중지됨");
    Ok(())
}

fn status() -> Result<()> {
    if cfg!(target_os = "macos") {
        let output = Command::new("launchctl")
            .args(["list", "com.filepipeline.watch"])
            .output()
            .context("launchctl list 실패")?;
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        let output = Command::new("systemctl")
            .args(["--user", "status", "file-pipeline.service"])
            .output()
            .context("systemctl status 실패")?;
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
    Ok(())
}

fn logs() -> Result<()> {
    if cfg!(target_os = "macos") {
        let log_path = "/tmp/filepipeline.stderr.log";
        println!("로그 파일: {}", log_path);
        Command::new("tail")
            .args(["-f", "-n", "50", log_path])
            .status()
            .context("tail 실패")?;
    } else {
        Command::new("journalctl")
            .args(["--user", "-u", "file-pipeline.service", "-f", "-n", "50"])
            .status()
            .context("journalctl 실패")?;
    }
    Ok(())
}

fn uninstall() -> Result<()> {
    let _ = stop();

    if cfg!(target_os = "macos") {
        let plist = service_dir().join("com.filepipeline.watch.plist");
        let _ = std::fs::remove_file(&plist);
        info!("launchd plist 삭제: {:?}", plist);
    } else {
        Command::new("systemctl")
            .args(["--user", "disable", "file-pipeline.service"])
            .output()
            .ok();
        let unit = service_dir().join("file-pipeline.service");
        let _ = std::fs::remove_file(&unit);
        Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output()
            .ok();
        info!("systemd unit 삭제: {:?}", unit);
    }
    Ok(())
}
