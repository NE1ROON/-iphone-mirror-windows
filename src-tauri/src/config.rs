// src-tauri/src/config.rs
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use crate::window_manager::ReceiverType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub receiver: ReceiverType,
    pub window_position: Option<WindowPosition>,
    pub window_size: Option<WindowSize>,
    pub auto_start_mirror: bool,
    pub click_through: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            receiver: ReceiverType::Bezel,
            window_position: Some(WindowPosition { x: 50, y: 50 }),
            window_size: Some(WindowSize { width: 400, height: 720 }),
            auto_start_mirror: true,
            click_through: false,
        }
    }
}

impl AppConfig {
    fn store_path(app: &AppHandle) -> PathBuf {
        let mut path = app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push("settings.json");
        path
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Загружается через tauri-plugin-store, здесь заглушка
        Ok(Self::default())
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Сохраняется через tauri-plugin-store
        Ok(())
    }
}