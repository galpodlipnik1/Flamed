#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod audio;
mod error;
mod lol;
mod secrets;
mod settings;
mod speech;

use error::AppError;
use secrets::{KeyringSecretStore, SecretStore};
use settings::{AiProvider, LoadSettingsResult, Settings, SettingsState, UiSettings};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State, WebviewWindow, WindowEvent,
};

#[tauri::command]
async fn load_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
) -> Result<LoadSettingsResult, String> {
    let secrets = KeyringSecretStore;
    let (settings, loaded) =
        settings::load_settings_from_disk(&app, &secrets).map_err(String::from)?;
    state.set(settings.clone()).await;
    Ok(loaded)
}

#[tauri::command]
async fn save_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    settings: Settings,
) -> Result<UiSettings, String> {
    let settings = settings.normalize();
    settings::save_settings_to_disk(&app, &settings).map_err(String::from)?;
    state.set(settings.clone()).await;

    let secrets = KeyringSecretStore;
    let saved_api_key_providers = saved_api_key_providers(&secrets).map_err(String::from)?;
    Ok(UiSettings::from_settings(settings, saved_api_key_providers))
}

#[tauri::command]
async fn generate_insult(
    state: State<'_, SettingsState>,
    killer: String,
    champion: String,
    death_streak: u32,
    kda: String,
    game_time_seconds: f64,
) -> Result<String, String> {
    let settings = state.get().await;
    let secrets = KeyringSecretStore;
    let api_key = secrets
        .get_api_key(settings.provider)
        .map_err(String::from)?
        .ok_or_else(|| {
            String::from(AppError::Ai(format!(
                "{} API key is not set.",
                settings.provider.label()
            )))
        })?;

    ai::generate_insult_with_settings(
        &settings,
        &api_key,
        ai::RoastRequest {
            killer: &killer,
            champion: &champion,
            death_streak,
            kda: &kda,
            game_time_seconds,
        },
    )
    .await
    .map_err(String::from)
}

#[tauri::command]
fn play_death_sound(app: AppHandle, volume: f32) -> Result<(), String> {
    audio::play_death_sound(&app, volume).map_err(String::from)
}

#[tauri::command]
async fn set_provider_api_key(
    state: State<'_, SettingsState>,
    provider: AiProvider,
    api_key: String,
) -> Result<UiSettings, String> {
    let secrets = KeyringSecretStore;
    secrets
        .set_api_key(provider, &api_key)
        .map_err(String::from)?;

    let saved_api_key_providers = saved_api_key_providers(&secrets).map_err(String::from)?;
    if !saved_api_key_providers.contains(&provider) {
        return Err(String::from(AppError::Secret(format!(
            "{} API key was saved but could not be found afterward.",
            provider.label()
        ))));
    }

    let settings = state.get().await;
    Ok(UiSettings::from_settings(settings, saved_api_key_providers))
}

#[tauri::command]
async fn clear_provider_api_key(
    state: State<'_, SettingsState>,
    provider: AiProvider,
) -> Result<UiSettings, String> {
    let secrets = KeyringSecretStore;
    secrets.clear_api_key(provider).map_err(String::from)?;

    let settings = state.get().await;
    let mut saved_api_key_providers = saved_api_key_providers(&secrets).map_err(String::from)?;
    saved_api_key_providers.retain(|saved_provider| *saved_provider != provider);

    Ok(UiSettings::from_settings(settings, saved_api_key_providers))
}

#[tauri::command]
async fn test_saved_api_key(provider: AiProvider) -> Result<bool, String> {
    let secrets = KeyringSecretStore;
    let api_key = secrets
        .get_api_key(provider)
        .map_err(String::from)?
        .ok_or_else(|| {
            String::from(AppError::Ai(format!(
                "{} API key is not set.",
                provider.label()
            )))
        })?;

    ai::test_api_key(provider, &api_key)
        .await
        .map_err(String::from)
}

#[tauri::command]
fn provider_options() -> &'static [ai::ProviderOption] {
    ai::provider_options()
}

fn saved_api_key_providers(secrets: &impl SecretStore) -> Result<Vec<AiProvider>, AppError> {
    [
        AiProvider::Gemini,
        AiProvider::OpenAi,
        AiProvider::Anthropic,
    ]
    .into_iter()
    .filter_map(|provider| match secrets.has_api_key(provider) {
        Ok(true) => Some(Ok(provider)),
        Ok(false) => None,
        Err(error) => Some(Err(error)),
    })
    .collect()
}

fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "open", "Open Flamed", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("missing application icon")?;

    TrayIconBuilder::new()
        .tooltip("Flamed")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_settings(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn position_overlay(app: &AppHandle) -> Result<(), String> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window not found".to_string())?;

    let monitor = overlay
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "primary monitor not found".to_string())?;

    let size = monitor.size();
    let overlay_height = 260_i32;

    overlay
        .set_size(tauri::PhysicalSize::new(size.width, overlay_height as u32))
        .map_err(|e| e.to_string())?;

    overlay
        .set_position(tauri::PhysicalPosition::new(
            monitor.position().x,
            monitor.position().y + size.height as i32 - overlay_height,
        ))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(windows)]
fn install_overlay_hit_test(window: &WebviewWindow) -> Result<(), String> {
    use std::sync::OnceLock;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, GetWindowLongPtrW, GetWindowRect, SetWindowLongPtrW,
        GWLP_WNDPROC, HTCLIENT, HTTRANSPARENT, WM_NCHITTEST, WNDPROC,
    };

    static OLD_PROC: OnceLock<isize> = OnceLock::new();

    unsafe extern "system" fn proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_NCHITTEST {
            let x = (lparam.0 & 0xffff) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
            let mut rect = RECT::default();

            if unsafe { GetWindowRect(hwnd, &mut rect).is_ok() } {
                let in_close_region =
                    x >= rect.right - 64 && x <= rect.right && y >= rect.top && y <= rect.top + 64;

                if in_close_region {
                    return LRESULT(HTCLIENT as isize);
                }

                return LRESULT(HTTRANSPARENT as isize);
            }
        }

        if let Some(old) = OLD_PROC.get().copied() {
            let old_proc: WNDPROC = unsafe { std::mem::transmute(old) };
            return unsafe { CallWindowProcW(old_proc, hwnd, msg, wparam, lparam) };
        }

        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    unsafe {
        let old = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
        let _ = OLD_PROC.set(old);
        let new_proc: WNDPROC = Some(proc);
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, std::mem::transmute(new_proc));
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let secrets = KeyringSecretStore;
            let initial_settings = match settings::load_settings_from_disk(&app_handle, &secrets) {
                Ok((settings, _)) => settings,
                Err(error) => {
                    eprintln!("[settings] failed to load settings: {error}");
                    Settings::default()
                }
            };
            app.manage(SettingsState::new(initial_settings));

            setup_tray(app)?;
            position_overlay(&app_handle)?;

            if let Some(overlay) = app_handle.get_webview_window("overlay") {
                let _ = overlay.hide();

                #[cfg(windows)]
                install_overlay_hit_test(&overlay)?;
            }

            let state = app.state::<SettingsState>().inner.clone();
            tauri::async_runtime::spawn(lol::poll_lol(app_handle, state));

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "settings" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            generate_insult,
            play_death_sound,
            set_provider_api_key,
            clear_provider_api_key,
            test_saved_api_key,
            provider_options
        ])
        .run(tauri::generate_context!())
        .expect("error while running Flamed");
}
