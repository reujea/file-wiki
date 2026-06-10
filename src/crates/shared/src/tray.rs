//! Windows 시스템 트레이 아이콘 관리
//!
//! `pipeline watch --tray` 옵션으로 활성화.
//! 트레이 메뉴: Watch 상태 / Stats / Open Inbox / Quit

#[cfg(windows)]
pub mod windows_tray {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

    /// 트레이 아이콘 실행 결과
    pub enum TrayAction {
        Quit,
        Stats,
        OpenInbox,
    }

    /// 트레이 아이콘 매니저
    pub struct TrayManager {
        _tray: TrayIcon,
        quit_flag: Arc<AtomicBool>,
        quit_item_id: tray_icon::menu::MenuId,
        stats_item_id: tray_icon::menu::MenuId,
        inbox_item_id: tray_icon::menu::MenuId,
    }

    impl TrayManager {
        pub fn new() -> anyhow::Result<Self> {
            let menu = Menu::new();

            let status = MenuItem::new("File Pipeline - 감시 중", false, None);
            let separator = tray_icon::menu::PredefinedMenuItem::separator();
            let stats_item = MenuItem::new("통계 보기", true, None);
            let inbox_item = MenuItem::new("Inbox 열기", true, None);
            let quit_item = MenuItem::new("종료", true, None);

            let stats_id = stats_item.id().clone();
            let inbox_id = inbox_item.id().clone();
            let quit_id = quit_item.id().clone();

            menu.append(&status)?;
            menu.append(&separator)?;
            menu.append(&stats_item)?;
            menu.append(&inbox_item)?;
            menu.append(&quit_item)?;

            // 아이콘: 16x16 빈 RGBA (실제 사용 시 .ico 파일로 교체)
            let icon = Icon::from_rgba(vec![0u8; 16 * 16 * 4], 16, 16)?;

            let tray = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip("File Pipeline")
                .with_icon(icon)
                .build()?;

            Ok(Self {
                _tray: tray,
                quit_flag: Arc::new(AtomicBool::new(false)),
                quit_item_id: quit_id,
                stats_item_id: stats_id,
                inbox_item_id: inbox_id,
            })
        }

        /// 메뉴 이벤트 폴링 (비차단)
        pub fn poll_action(&self) -> Option<TrayAction> {
            if let Ok(event) = MenuEvent::receiver().try_recv() {
                if event.id == self.quit_item_id {
                    self.quit_flag.store(true, Ordering::Relaxed);
                    return Some(TrayAction::Quit);
                }
                if event.id == self.stats_item_id {
                    return Some(TrayAction::Stats);
                }
                if event.id == self.inbox_item_id {
                    return Some(TrayAction::OpenInbox);
                }
            }
            None
        }

        pub fn should_quit(&self) -> bool {
            self.quit_flag.load(Ordering::Relaxed)
        }
    }
}

#[cfg(not(windows))]
pub mod windows_tray {
    pub struct TrayManager;
    pub enum TrayAction { Quit, Stats, OpenInbox }

    impl TrayManager {
        pub fn new() -> anyhow::Result<Self> { Ok(Self) }
        pub fn poll_action(&self) -> Option<TrayAction> { None }
        pub fn should_quit(&self) -> bool { false }
    }
}
