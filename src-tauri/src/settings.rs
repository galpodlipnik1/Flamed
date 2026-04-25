use crate::{
    ai,
    error::{AppError, AppResult},
    secrets::SecretStore,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub provider: AiProvider,
    #[serde(default = "default_selected_model")]
    pub selected_model: String,
    #[serde(default)]
    pub censorship_enabled: bool,
    #[serde(default)]
    pub insult_preset: InsultPreset,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default = "default_volume")]
    pub speech_volume: f32,
    #[serde(default = "default_overlay_enabled")]
    pub overlay_enabled: bool,
    #[serde(default)]
    pub speech_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: AiProvider::Gemini,
            selected_model: default_selected_model(),
            censorship_enabled: false,
            insult_preset: InsultPreset::default(),
            volume: default_volume(),
            speech_volume: default_volume(),
            overlay_enabled: default_overlay_enabled(),
            speech_enabled: false,
        }
    }
}

impl Settings {
    pub fn normalize(mut self) -> Self {
        self.volume = self.volume.clamp(0.0, 1.0);
        self.speech_volume = self.speech_volume.clamp(0.0, 1.0);
        self.selected_model = ai::normalize_provider_model(self.provider, self.selected_model);
        if self.provider != AiProvider::Gemini {
            self.speech_enabled = false;
        }
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    Gemini,
    OpenAi,
    Anthropic,
}

impl Default for AiProvider {
    fn default() -> Self {
        Self::Gemini
    }
}

impl AiProvider {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Gemini => "gemini",
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Gemini => "Gemini",
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Anthropic",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    Frontier,
    Mid,
    Budget,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InsultPreset {
    Warmup,
    Salty,
    Brutal,
    Nuclear,
}

impl Default for InsultPreset {
    fn default() -> Self {
        Self::Brutal
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UiSettings {
    pub provider: AiProvider,
    pub selected_model: String,
    pub censorship_enabled: bool,
    pub insult_preset: InsultPreset,
    pub volume: f32,
    pub speech_volume: f32,
    pub overlay_enabled: bool,
    pub speech_enabled: bool,
    pub saved_api_key_providers: Vec<AiProvider>,
}

impl UiSettings {
    pub fn from_settings(settings: Settings, saved_api_key_providers: Vec<AiProvider>) -> Self {
        Self {
            provider: settings.provider,
            selected_model: settings.selected_model,
            censorship_enabled: settings.censorship_enabled,
            insult_preset: settings.insult_preset,
            volume: settings.volume,
            speech_volume: settings.speech_volume,
            overlay_enabled: settings.overlay_enabled,
            speech_enabled: settings.speech_enabled,
            saved_api_key_providers,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadSettingsResult {
    pub settings: UiSettings,
    pub warning: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SettingsFile {
    #[serde(default)]
    gemini_api_key: Option<String>,
    #[serde(default)]
    provider: AiProvider,
    #[serde(default = "default_selected_model")]
    selected_model: String,
    #[serde(default)]
    censorship_enabled: bool,
    #[serde(default)]
    insult_preset: InsultPreset,
    #[serde(default = "default_volume")]
    volume: f32,
    #[serde(default = "default_volume")]
    speech_volume: f32,
    #[serde(default = "default_overlay_enabled")]
    overlay_enabled: bool,
    #[serde(default)]
    speech_enabled: bool,
}

impl SettingsFile {
    fn into_settings(self) -> (Settings, Option<String>) {
        (
            Settings {
                provider: self.provider,
                selected_model: self.selected_model,
                censorship_enabled: self.censorship_enabled,
                insult_preset: self.insult_preset,
                volume: self.volume,
                speech_volume: self.speech_volume,
                overlay_enabled: self.overlay_enabled,
                speech_enabled: self.speech_enabled,
            }
            .normalize(),
            self.gemini_api_key,
        )
    }
}

pub type SharedSettings = Arc<RwLock<Settings>>;

pub struct SettingsState {
    pub inner: SharedSettings,
}

impl SettingsState {
    pub fn new(settings: Settings) -> Self {
        Self {
            inner: Arc::new(RwLock::new(settings)),
        }
    }

    pub async fn get(&self) -> Settings {
        self.inner.read().await.clone()
    }

    pub async fn set(&self, settings: Settings) {
        *self.inner.write().await = settings;
    }
}

fn default_volume() -> f32 {
    0.8
}

fn default_selected_model() -> String {
    ai::default_model_for_provider(AiProvider::Gemini).to_string()
}

fn default_overlay_enabled() -> bool {
    true
}

fn settings_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Settings(error.to_string()))?;
    Ok(dir.join("settings.json"))
}

pub fn load_settings_from_disk(
    app: &AppHandle,
    secrets: &impl SecretStore,
) -> AppResult<(Settings, LoadSettingsResult)> {
    let path = settings_path(app)?;
    load_settings_from_path(&path, secrets)
}

pub fn load_settings_from_path(
    path: &Path,
    secrets: &impl SecretStore,
) -> AppResult<(Settings, LoadSettingsResult)> {
    let mut warning = None;

    if !path.exists() {
        let settings = Settings::default();
        save_settings_to_path(path, &settings)?;
        let saved_api_key_providers = saved_api_key_providers(secrets)?;
        return Ok((
            settings.clone(),
            LoadSettingsResult {
                settings: UiSettings::from_settings(settings, saved_api_key_providers),
                warning,
            },
        ));
    }

    let raw = fs::read_to_string(path)?;
    let parsed = match serde_json::from_str::<SettingsFile>(&raw) {
        Ok(parsed) => parsed,
        Err(error) => {
            let backup = backup_corrupted_settings(path)?;
            let settings = Settings::default();
            save_settings_to_path(path, &settings)?;
            warning = Some(format!(
        "Settings were unreadable and have been reset. A backup was saved to {}. Parse error: {error}",
        backup.display()
      ));

            let saved_api_key_providers = saved_api_key_providers(secrets)?;
            return Ok((
                settings.clone(),
                LoadSettingsResult {
                    settings: UiSettings::from_settings(settings, saved_api_key_providers),
                    warning,
                },
            ));
        }
    };

    let (settings, legacy_api_key) = parsed.into_settings();

    if let Some(api_key) = legacy_api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        secrets.set_api_key(AiProvider::Gemini, api_key)?;
        save_settings_to_path(path, &settings)?;
    }

    let saved_api_key_providers = saved_api_key_providers(secrets)?;
    Ok((
        settings.clone(),
        LoadSettingsResult {
            settings: UiSettings::from_settings(settings, saved_api_key_providers),
            warning,
        },
    ))
}

pub fn save_settings_to_disk(app: &AppHandle, settings: &Settings) -> AppResult<()> {
    let path = settings_path(app)?;
    save_settings_to_path(&path, settings)
}

pub fn save_settings_to_path(path: &Path, settings: &Settings) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let raw = serde_json::to_string_pretty(settings)?;
    fs::write(path, raw)?;
    Ok(())
}

fn backup_corrupted_settings(path: &Path) -> AppResult<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::Settings(error.to_string()))?
        .as_secs();

    let backup = path.with_extension(format!("json.{timestamp}.bak"));
    fs::rename(path, &backup)?;
    Ok(backup)
}

fn saved_api_key_providers(secrets: &impl SecretStore) -> AppResult<Vec<AiProvider>> {
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

#[cfg(test)]
mod tests {
    use super::{load_settings_from_path, save_settings_to_path, AiProvider, Settings};
    use crate::{error::AppResult, secrets::SecretStore};
    use std::{
        cell::RefCell,
        collections::HashMap,
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[derive(Default)]
    struct MemorySecrets {
        api_keys: RefCell<HashMap<AiProvider, String>>,
    }

    impl SecretStore for MemorySecrets {
        fn get_api_key(&self, provider: AiProvider) -> AppResult<Option<String>> {
            Ok(self.api_keys.borrow().get(&provider).cloned())
        }

        fn set_api_key(&self, provider: AiProvider, api_key: &str) -> AppResult<()> {
            self.api_keys
                .borrow_mut()
                .insert(provider, api_key.to_string());
            Ok(())
        }

        fn clear_api_key(&self, provider: AiProvider) -> AppResult<()> {
            self.api_keys.borrow_mut().remove(&provider);
            Ok(())
        }
    }

    fn temp_settings_path(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir()
            .join("flamed-tests")
            .join(format!("{test_name}-{unique}"))
            .join("settings.json")
    }

    #[test]
    fn migrates_plaintext_api_key_out_of_settings_file() {
        let path = temp_settings_path("migrates-plaintext-key");
        fs::create_dir_all(path.parent().expect("parent")).expect("create temp dir");
        fs::write(
      &path,
      r#"{"gemini_api_key":"secret-key","censorship_enabled":true,"volume":1.25,"overlay_enabled":false}"#,
    )
    .expect("write settings");

        let secrets = MemorySecrets::default();
        let (settings, loaded) = load_settings_from_path(&path, &secrets).expect("load settings");
        let rewritten = fs::read_to_string(&path).expect("read rewritten settings");

        assert_eq!(
            secrets
                .get_api_key(AiProvider::Gemini)
                .expect("get api key")
                .as_deref(),
            Some("secret-key")
        );
        assert!(!rewritten.contains("gemini_api_key"));
        assert!(settings.censorship_enabled);
        assert_eq!(settings.volume, 1.0);
        assert!(!settings.overlay_enabled);
        assert_eq!(
            loaded.settings.saved_api_key_providers,
            vec![AiProvider::Gemini]
        );
    }

    #[test]
    fn backs_up_corrupted_settings_and_restores_defaults() {
        let path = temp_settings_path("backs-up-corrupted-settings");
        let dir = path.parent().expect("parent").to_path_buf();
        fs::create_dir_all(&dir).expect("create temp dir");
        fs::write(&path, "{not json").expect("write bad settings");

        let secrets = MemorySecrets::default();
        let (settings, loaded) = load_settings_from_path(&path, &secrets).expect("load settings");
        let backups = fs::read_dir(&dir)
            .expect("read temp dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".bak"))
            .count();

        assert_eq!(settings.volume, Settings::default().volume);
        assert_eq!(
            settings.overlay_enabled,
            Settings::default().overlay_enabled
        );
        assert_eq!(backups, 1);
        assert!(loaded.warning.is_some());
    }

    #[test]
    fn normalizes_volume_to_supported_range() {
        let low = Settings {
            volume: -2.0,
            ..Settings::default()
        }
        .normalize();
        let high = Settings {
            volume: 4.0,
            ..Settings::default()
        }
        .normalize();

        assert_eq!(low.volume, 0.0);
        assert_eq!(high.volume, 1.0);
    }

    #[test]
    fn disables_speech_for_non_gemini_providers() {
        let settings = Settings {
            provider: AiProvider::OpenAi,
            speech_enabled: true,
            ..Settings::default()
        }
        .normalize();

        assert!(!settings.speech_enabled);
    }

    #[test]
    fn saves_settings_without_secret_fields() {
        let path = temp_settings_path("saves-without-secret-fields");
        save_settings_to_path(&path, &Settings::default()).expect("save settings");
        let raw = fs::read_to_string(path).expect("read settings");

        assert!(!raw.contains("gemini_api_key"));
    }
}
