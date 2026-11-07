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
    Bezel,
    Reflector,
    LonelyScreen,
}

impl Default for ReceiverType {
    fn default() -> Self {
        Self::Bezel
    }
}

impl ReceiverType {
    pub fn window_class(&self) -> &'static str {
        match self {
            Self::Bezel => "Chrome_WidgetWin_1", // Bezel на Electron
            Self::Reflector => "ReflectorWindowClass", // Reflector 4
            Self::LonelyScreen => "LonelyScreenWindowClass",
        }
    }

    pub fn process_name(&self) -> &'static str {
        match self {
            Self::Bezel => "Bezel.exe",
            Self::Reflector => "Reflector 4.exe",
            Self::LonelyScreen => "LonelyScreen.exe",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Bezel => "Bezel (бесплатно, open source)",
            Self::Reflector => "Reflector 4 (платный, триал 7 дней)",
            Self::LonelyScreen => "LonelyScreen (бесплатно, устарел)",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Bezel, Self::Reflector, Self::LonelyScreen]
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReceiverConfig {
    pub r#type: ReceiverType,
    pub display_name: String,
    pub process_name: String,
    pub window_class: String,
}

impl ReceiverConfig {
    pub fn all() -> Vec<Self> {
        ReceiverType::all()
            .into_iter()
            .map(|t| Self {
                display_name: t.display_name().to_string(),
                process_name: t.process_name().to_string(),
                window_class: t.window_class().to_string(),
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
        // Для теста создаём заглушку без app handle
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

    /// Запускает мониторинг окна (вызывается при старте)
    pub fn start_monitoring(&self) {
        // Первичный поиск
        let _ = self.find_and_style_window();
    }

    /// Основная функция: находит окно ресивера и применяет стили
    pub fn ensure_window_style(&self) -> Result<(), String> {
        self.find_and_style_window()
    }

    fn find_and_style_window(&self) -> Result<(), String> {
        let hwnd = self.find_receiver_window()?;
        if hwnd.0 == 0 {
            return Ok(()); // Окно ещё не создано
        }

        // Проверяем, не обрабатывали ли уже это окно
        let mut target = self.target_hwnd.lock().unwrap();
        if *target == Some(hwnd) {
            return Ok(());
        }

        // Сохраняем оригинальные стили
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as i32;
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as i32;

            *self.original_style.lock().unwrap() = Some(style);
            *self.original_ex_style.lock().unwrap() = Some(ex_style);

            // Применяем новые стили
            let new_ex_style = WS_EX_TOPMOST.0 | WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0;
            let new_style = WS_POPUP.0 | WS_VISIBLE.0;

            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style as isize);
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);

            // Принудительно обновляем окно
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }

        *target = Some(hwnd);
        info!("Styled mirror window: {:?}", hwnd);
        Ok(())
    }

    fn find_receiver_window(&self) -> Result<HWND, String> {
        unsafe {
            // Ищем по классу окна
            let class_name: Vec<u16> = OsStr::new(self.receiver.window_class()).encode_wide().chain(Some(0)).collect();
            let mut hwnd = FindWindowW(None, None);
            while hwnd.0 != 0 {
                let mut buf = [0u16; 256];
                let len = GetClassNameW(hwnd, &mut buf);
                if len > 0 && &buf[..len as usize] == &class_name[..class_name.len()-1] {
                    // Проверяем, что окно видимо и принадлежит нужному процессу
                    if IsWindowVisible(hwnd).as_bool() {
                        return Ok(hwnd);
                    }
                }
                hwnd = FindWindowExW(None, Some(hwnd), None, None);
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

    /// Восстанавливает оригинальные стили при выходе
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