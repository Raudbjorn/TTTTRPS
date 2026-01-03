//! Claude Bridge - Tauri application entry point.
//!
//! This application provides a bridge between local processes and Claude Desktop
//! via Chrome DevTools Protocol (CDP).

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    claude_bridge_tauri::run();
}
