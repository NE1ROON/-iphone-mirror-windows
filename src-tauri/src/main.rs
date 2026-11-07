// src-tauri/src/main.rs
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_store::StoreExt;
use tokio::time::interval;
use tracing::{error, info, warn};
use window_manager::{MirrorWindowManager, ReceiverType};

mod window_manager;
mod config;

use config::{AppConfig, ReceiverConfig};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TrayState {
    visible: bool,
    receiver: ReceiverType,
}

#[derive(Default)]
struct AppState {
    manager: Mutex<Option<MirrorWindowManager>>,
    config: Mutex<AppConfig>,
    tray_state: Mutex<TrayState>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Инициализация логирования
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Загружаем конфиг
    let config = AppConfig::load().unwrap_or_default();
    let receiver = config.receiver.clone();

    let app_state = Arc::new(AppState {
        manager: Mutex::new(None),
        config: Mutex::new(config),
        tray_state: Mutex::new(TrayState { visible: false, receiver }),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(app_state.clone())
        .setup(move |app| {
            let handle = app.handle().clone();
            let state = app_state.clone();

            // Инициализируем менеджер окон
            let mut manager = MirrorWindowManager::new(handle.clone(), receiver.clone());
            manager.start_monitoring();
            *state.manager.lock().unwrap() = Some(manager);

            // Создаём трей-меню
            let show_hide = MenuItem::with_id(&handle, "toggle", "Скрыть окно", true, None::<&str>)?;
            let settings = MenuItem::with_id(&handle, "settings", "Настройки…", true, None::<&str>)?;
            let autostart = MenuItem::with_id(&handle, "autostart", "Автозапуск", true, None::<&str>)?;
            let quit = MenuItem::with_id(&handle, "quit", "Выход", true, None::<&str>)?;

            let menu = Menu::with_items(&handle, &[&show_hide, &settings, &autostart, &quit])?;

            TrayIconBuilder::new()
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?)
                .menu(&menu)
                .tooltip("iPhone Mirror — клик для показа/скрытия")
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let _ = toggle_visibility(tray.app_handle(), state.clone());
                    }
                })
                .build(&handle)?;

            // Обновляем состояние автозапуска в меню
            update_autostart_menu(&handle, &state).ok();

            // Запускаем периодическую проверку окна Bezel
            let handle_clone = handle.clone();
            let state_clone = state.clone();
            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(2));
                loop {
                    interval.tick().await;
                    if let Some(manager) = state_clone.manager.lock().unwrap().as_mut() {
                        if let Err(e) = manager.ensure_window_style() {
                            warn!("Window style check failed: {}", e);
                        }
                    }
                }
            });

            // Если запущены с --hidden, не показываем окно настроек
            if std::env::args().any(|a| a == "--hidden") {
                info!("Started hidden (autostart)");
            } else {
                // Показываем окно настроек при первом запуске
                if let Some(window) = handle.get_webview_window("settings") {
                    window.show().ok();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Скрываем вместо закрытия
                if window.label() == "settings" {
                    api.prevent_close();
                    window.hide().ok();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            toggle_visibility,
            open_settings,
            get_config,
            update_config,
            get_receivers,
            test_receiver,
            set_autostart,
            get_autostart,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

// ===== Команды для фронтенда =====

#[tauri::command]
async fn toggle_visibility(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let mut tray_state = state.tray_state.lock().unwrap();
    let manager = state.manager.lock().unwrap();

    if let Some(mgr) = manager.as_ref() {
        if tray_state.visible {
            mgr.hide_mirror_window()?;
            tray_state.visible = false;
            // Обновляем текст меню
            if let Some(item) = app.tray_by_id("main").and_then(|t| t.menu()).and_then(|m| m.get("toggle")) {
                let _ = item.set_text("Показать окно");
            }
        } else {
            mgr.show_mirror_window()?;
            tray_state.visible = true;
            if let Some(item) = app.tray_by_id("main").and_then(|t| t.menu()).and_then(|m| m.get("toggle")) {
                let _ = item.set_text("Скрыть окно");
            }
        }
    }
    Ok(tray_state.visible)
}

#[tauri::command]
async fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().ok();
        window.set_focus().ok();
    }
    Ok(())
}

#[tauri::command]
async fn get_config(state: State<'_, Arc<AppState>>) -> Result<AppConfig, String> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
async fn update_config(app: AppHandle, state: State<'_, Arc<AppState>>, config: AppConfig) -> Result<(), String> {
    let mut current = state.config.lock().unwrap();
    let old_receiver = current.receiver.clone();
    *current = config.clone();
    current.save().map_err(|e| e.to_string())?;

    // Если сменился ресивер — перезапускаем менеджер
    if old_receiver != config.receiver {
        let mut manager = state.manager.lock().unwrap();
        if let Some(mgr) = manager.as_mut() {
            mgr.set_receiver(config.receiver.clone());
        }
        // Обновляем трей-стейт
        state.tray_state.lock().unwrap().receiver = config.receiver;
    }
    Ok(())
}

#[tauri::command]
async fn get_receivers() -> Result<Vec<ReceiverConfig>, String> {
    Ok(ReceiverConfig::all())
}

#[tauri::command]
async fn test_receiver(receiver: ReceiverType) -> Result<bool, String> {
    let mgr = MirrorWindowManager::new_test(receiver);
    Ok(mgr.find_window().is_some())
}

#[tauri::command]
async fn set_autostart(app: AppHandle, enable: bool) -> Result<(), String> {
    let autostart = tauri_plugin_autostart::AutostartManager::new(&app);
    if enable {
        autostart.enable().map_err(|e| e.to_string())?;
    } else {
        autostart.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn get_autostart(app: AppHandle) -> Result<bool, String> {
    let autostart = tauri_plugin_autostart::AutostartManager::new(&app);
    Ok(autostart.is_enabled().unwrap_or(false))
}

#[tauri::command]
async fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

fn update_autostart_menu(app: &AppHandle, state: &Arc<AppState>) -> Result<(), String> {
    let autostart = tauri_plugin_autostart::AutostartManager::new(app);
    let enabled = autostart.is_enabled().unwrap_or(false);
    if let Some(menu) = app.tray_by_id("main").and_then(|t| t.menu()) {
        if let Some(item) = menu.get("autostart") {
            let _ = item.set_text(if enabled { "✓ Автозапуск" } else { "Автозапуск" });
        }
    }
    Ok(())
}