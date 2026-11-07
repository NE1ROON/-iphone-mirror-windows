// src-tauri/src/window_manager.rs
use std::{
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    ptr::null_mut,
    sync::Mutex,
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
use tracing::{debug, info, warn};
use windows::{
    Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    Win32::UI::Shell::{SHAppBarMessage, ABM_NEW, ABM_REMOVE, ABM_SETPOS, APPBARDATA, ABE_LEFT, ABE_TOP, ABE_RIGHT, ABE_BOTTOM},
    Win32::UI::WindowsAndMessaging::{
        FindWindowExW, FindWindowW, GetClassNameW, GetWindowLongPtrW, GetWindowRect, GetWindowTextW,
        IsWindowVisible, SetWindowLongPtrW, SetWindowPos, ShowWindow, SW_HIDE, SW_SHOW, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE, GWL_EXSTYLE, GWL_STYLE,
        HWND_TOPMOST, HWND_NOTOPMOST,
    },
    Win32::System::Threading::{GetCurrentProcessId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    Win32::System::ProcessStatus::GetModuleFileNameExW,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReceiverType {
    MirrorDrop,
    Bezel,
    Reflector,
    LonelyScreen,
    AirPlayWindows,
    AirPlayReceiver,
}

impl Default for ReceiverType {
    fn default() -> Self {
        Self::MirrorDrop
    }
}

impl ReceiverType {
    /// Возможные классы окон для каждого ресивера (проверяются по очереди)
    pub fn window_classes(&self) -> &'static [&'static str] {
        match self {
            // MirrorDrop — нативное Windows приложение, класс может варьироваться
            Self::MirrorDrop => &[
                "MirrorDropWindow",      // предполагаемый класс
                "MirrorDrop",            // альтернатива
                "Windows.UI.Core.CoreWindow", // UWP/WinUI 3
                "Chrome_WidgetWin_1",    // если на Electron
            ],
            // Bezel — только macOS, но на случай если запустят через виртуалку/эмулятор
            Self::Bezel => &["Chrome_WidgetWin_1"],
            // Reflector 4
            Self::Reflector => &["ReflectorWindowClass", "Chrome_WidgetWin_1"],
            // LonelyScreen
            Self::LonelyScreen => &["LonelyScreenWindowClass"],
            // AirPlay-Windows (Thomas-lab17)
            Self::AirPlayWindows => &["AirPlayReceiverWindow", "Chrome_WidgetWin_1", "Windows.UI.Core.CoreWindow"],
            // AirPlay-Receiver (ZC635)
            Self::AirPlayReceiver => &["AirPlayReceiverWindow", "Chrome_WidgetWin_1"],
        }
    }

    pub fn process_names(&self) -> &'static [&'static str] {
        match self {
            Self::MirrorDrop => &["MirrorDrop.exe", "mirrordrop.exe"],
            Self::Bezel => &["Bezel.exe"],
            Self::Reflector => &["Reflector 4.exe", "Reflector.exe"],
            Self::LonelyScreen => &["LonelyScreen.exe"],
            Self::AirPlayWindows => &["AirPlayReceiver.exe", "AirPlay-Windows.exe"],
            Self::AirPlayReceiver => &["AirPlayReceiver.exe"],
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::MirrorDrop => "MirrorDrop (бесплатно, open source, рекомендуется)",
            Self::Bezel => "Bezel (только macOS)",
            Self::Reflector => "Reflector 4 (платный, триал 7 дней)",
            Self::LonelyScreen => "LonelyScreen (бесплатно, устарел)",
            Self::AirPlayWindows => "AirPlay-Windows (бесплатно, open source)",
            Self::AirPlayReceiver => "AirPlay-Receiver (бесплатно, open source)",
        }
    }

    pub fn download_url(&self) -> &'static str {
        match self {
            Self::MirrorDrop => "https://mirrordrop.app/windows",
            Self::Bezel => "https://nonstrict.eu/bezel/",
            Self::Reflector => "https://www.airsquirrels.com/reflector/",
            Self::LonelyScreen => "https://www.lonely-screen.com/",
            Self::AirPlayWindows => "https://github.com/Thomas-lab17/AirPlay-Windows/releases",
            Self::AirPlayReceiver => "https://github.com/ZC635/AirPlay-Receiver/releases",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::MirrorDrop,
            Self::AirPlayWindows,
            Self::AirPlayReceiver,
            Self::Reflector,
            Self::LonelyScreen,
            Self::Bezel,
        ]
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReceiverConfig {
    pub r#type: ReceiverType,
    pub display_name: String,
    pub process_names: Vec<String>,
    pub window_classes: Vec<String>,
    pub download_url: String,
}

impl ReceiverConfig {
    pub fn all() -> Vec<Self> {
        ReceiverType::all()
            .into_iter()
            .map(|t| Self {
                display_name: t.display_name().to_string(),
                process_names: t.process_names().iter().map(|s| s.to_string()).collect(),
                window_classes: t.window_classes().iter().map(|s| s.to_string()).collect(),
                download_url: t.download_url().to_string(),
                r#type: t,
            })
            .collect()
    }
}

pub struct MirrorWindowManager {
    app: AppHandle,
    receiver: ReceiverType,
    target_hwnd: Mutex<Option<HWND>>,
    original_style: Mutex<Option<i32>>,
    original_ex_style: Mutex<Option<i32>>,
    is_visible: Mutex<bool>,
}

impl MirrorWindowManager {
    pub fn new(app: AppHandle, receiver: ReceiverType) -> Self {
        Self {
            app,
            receiver,
            target_hwnd: Mutex::new(None),
            original_style: Mutex::new(None),
            original_ex_style: Mutex::new(None),
            is_visible: Mutex::new(false),
        }
    }

    pub fn new_test(receiver: ReceiverType) -> Self {
        use tauri::test::mock_app;
        Self {
            app: mock_app(),
            receiver,
            target_hwnd: Mutex::new(None),
            original_style: Mutex::new(None),
            original_ex_style: Mutex::new(None),
            is_visible: Mutex::new(false),
        }
    }

    pub fn set_receiver(&self, receiver: ReceiverType) {
        *self.target_hwnd.lock().unwrap() = None;
    }

    pub fn start_monitoring(&self) {
        let _ = self.find_and_style_window();
    }

    pub fn ensure_window_style(&self) -> Result<(), String> {
        self.find_and_style_window()
    }

    fn find_and_style_window(&self) -> Result<(), String> {
        let hwnd = self.find_receiver_window()?;
        if hwnd.0 == 0 {
            return Ok(());
        }

        let mut target = self.target_hwnd.lock().unwrap();
        if *target == Some(hwnd) {
            return Ok(());
        }

        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as i32;
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as i32;

            *self.original_style.lock().unwrap() = Some(style);
            *self.original_ex_style.lock().unwrap() = Some(ex_style);

            // Применяем новые стили: frameless, always-on-top, toolwindow (не в таскбаре)
            let new_ex_style = WS_EX_TOPMOST.0 | WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0;
            let new_style = WS_POPUP.0 | WS_VISIBLE.0;

            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style as isize);
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);

            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }

        *target = Some(hwnd);
        info!("Styled mirror window: {:?} for receiver: {:?}", hwnd, self.receiver);
        Ok(())
    }

    fn find_receiver_window(&self) -> Result<HWND, String> {
        unsafe {
            // Сначала пробуем найти по классам окон
            for class_name_str in self.receiver.window_classes() {
                let class_name: Vec<u16> = OsStr::new(class_name_str).encode_wide().chain(Some(0)).collect();
                let mut hwnd = FindWindowW(None, None);
                while hwnd.0 != 0 {
                    let mut buf = [0u16; 256];
                    let len = GetClassNameW(hwnd, &mut buf);
                    if len > 0 && &buf[..len as usize] == &class_name[..class_name.len()-1] {
                        if IsWindowVisible(hwnd).as_bool() {
                            // Дополнительная проверка: принадлежит ли окно нужному процессу
                            if self.verify_process(hwnd) {
                                return Ok(hwnd);
                            }
                        }
                    }
                    hwnd = FindWindowExW(None, Some(hwnd), None, None);
                }
            }

            // Fallback: поиск по имени процесса (если класс не совпал)
            self.find_by_process_name()
        }
    }

    fn verify_process(&self, hwnd: HWND) -> bool {
        unsafe {
            let mut pid: u32 = 0;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return false;
            }

            if let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                let mut exe_path = [0u16; 260];
                let len = GetModuleFileNameExW(process, None, &mut exe_path);
                let _ = windows::Win32::Foundation::CloseHandle(process);
                if len > 0 {
                    let path_str = String::from_utf16_lossy(&exe_path[..len as usize]);
                    let exe_name = std::path::Path::new(&path_str)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();

                    for proc_name in self.receiver.process_names() {
                        if exe_name == proc_name.to_lowercase() {
                            return true;
                        }
                    }
                }
            }
            // Если не смогли проверить процесс, разрешаем (окно может быть от другого процесса с тем же классом)
            true
        }
    }

    fn find_by_process_name(&self) -> Result<HWND, String> {
        unsafe {
            for proc_name in self.receiver.process_names() {
                let proc_name_wide: Vec<u16> = OsStr::new(proc_name).encode_wide().chain(Some(0)).collect();
                // Используем CreateToolhelp32Snapshot для поиска процесса
                // Но проще: ищем все окна и проверяем их процессы
                let mut hwnd = FindWindowW(None, None);
                while hwnd.0 != 0 {
                    if IsWindowVisible(hwnd).as_bool() {
                        if self.verify_process(hwnd) {
                            // Дополнительно проверяем заголовок окна
                            let mut title = [0u16; 256];
                            let len = GetWindowTextW(hwnd, &mut title);
                            if len > 0 {
                                let title_str = String::from_utf16_lossy(&title[..len as usize]);
                                // Исключаем служебные окна
                                if !title_str.is_empty() && title_str.len() > 2 {
                                    return Ok(hwnd);
                                }
                            }
                        }
                    }
                    hwnd = FindWindowExW(None, Some(hwnd), None, None);
                }
            }
        }
        Err("Window not found".into())
    }

    pub fn show_mirror_window(&self) -> Result<(), String> {
        let hwnd = self.find_receiver_window()?;
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
        *self.is_visible.lock().unwrap() = true;
        self.emit_state(true);
        Ok(())
    }

    pub fn hide_mirror_window(&self) -> Result<(), String> {
        if let Some(hwnd) = *self.target_hwnd.lock().unwrap() {
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
        *self.is_visible.lock().unwrap() = false;
        self.emit_state(false);
        Ok(())
    }

    fn emit_state(&self, visible: bool) {
        let _ = self.app.emit("mirror-window-state", serde_json::json!({ "visible": visible }));
    }

    pub fn restore_original_style(&self) {
        if let Some(hwnd) = *self.target_hwnd.lock().unwrap() {
            if let (Some(style), Some(ex_style)) = (
                *self.original_style.lock().unwrap(),
                *self.original_ex_style.lock().unwrap(),
            ) {
                unsafe {
                    SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style as isize);
                    SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED);
                }
            }
        }
    }
}

impl Drop for MirrorWindowManager {
    fn drop(&mut self) {
        self.restore_original_style();
    }
}