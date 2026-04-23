//! Persisted user settings.
//!
//! Lives in `%APPDATA%\DockDuo\config.json`. Written atomically via a
//! temp-file + rename so a crash mid-write can never leave a half-written
//! file on disk. The frontend reads/writes via the `get_config` /
//! `set_config` Tauri commands.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// Current on-disk schema. Bump + migrate if fields are renamed/removed.
const CONFIG_VERSION: u32 = 5;

/// Supported visual themes. Maps 1:1 to CSS variable bundles on the frontend.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Midnight,
    Daylight,
    Pastel,
    Retro,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Midnight
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub version: u32,
    pub theme: Theme,
    /// User has completed the onboarding flow at least once.
    pub onboarded: bool,
    /// Hide overlay when a fullscreen app (game, video) is focused.
    pub hide_on_fullscreen: bool,
    /// Persist which agent was last used per character so we auto-spawn it
    /// again rather than re-prompting every time.
    pub last_agent_bruce: Option<String>,
    pub last_agent_jazz: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            theme: Theme::default(),
            onboarded: false,
            hide_on_fullscreen: true,
            last_agent_bruce: None,
            last_agent_jazz: None,
        }
    }
}

/// In-memory cache so reads don't hit disk every time.
static CACHE: Lazy<Mutex<Option<AppConfig>>> = Lazy::new(|| Mutex::new(None));

fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not resolve %APPDATA%")?;
    Ok(base.join("DockDuo"))
}

fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.json"))
}

/// Read the config from disk (or return defaults). Cached after first read.
pub fn load() -> AppConfig {
    {
        let guard = CACHE.lock().unwrap();
        if let Some(cfg) = guard.as_ref() {
            return cfg.clone();
        }
    }
    let cfg = read_from_disk().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "config read failed — using defaults");
        AppConfig::default()
    });
    *CACHE.lock().unwrap() = Some(cfg.clone());
    cfg
}

fn read_from_disk() -> Result<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    // Parse leniently: the bubble-only migration dropped
    // `use_embedded_terminal` and `embedded_ui_mode`. `#[serde(default)]` on
    // the struct lets us ignore any extras the old binary left behind.
    let cfg: AppConfig = serde_json::from_str(&raw)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(migrate(cfg))
}

/// Migrate older schemas to the current one.
fn migrate(mut cfg: AppConfig) -> AppConfig {
    if cfg.version == CONFIG_VERSION {
        return cfg;
    }
    tracing::info!(
        from = cfg.version,
        to = CONFIG_VERSION,
        "config schema migrated"
    );
    cfg.version = CONFIG_VERSION;
    cfg
}

/// Atomic-write the config. Writes to `<path>.tmp` and renames, so a
/// partial write can never leave `config.json` corrupt.
pub fn save(cfg: &AppConfig) -> Result<()> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let target = config_path()?;
    let tmp = target.with_extension("json.tmp");

    let json = serde_json::to_string_pretty(cfg)?;
    fs::write(&tmp, json).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, &target)
        .with_context(|| format!("atomic rename → {}", target.display()))?;

    *CACHE.lock().unwrap() = Some(cfg.clone());
    Ok(())
}

/// Mutate the config in-place and persist. Used by setter commands.
pub fn update<F: FnOnce(&mut AppConfig)>(f: F) -> Result<AppConfig> {
    let mut cfg = load();
    f(&mut cfg);
    save(&cfg)?;
    Ok(cfg)
}

// ---------- Tauri commands ----------

#[tauri::command]
pub fn get_config() -> AppConfig {
    load()
}

#[tauri::command]
pub fn set_theme(theme: Theme) -> Result<AppConfig, String> {
    update(|c| c.theme = theme).map_err(|e| e.to_string())
}

/// Not a Tauri command — called from `fullscreen::set_hide_on_fullscreen`
/// which additionally flips overlay visibility immediately.
pub fn set_hide_on_fullscreen(enabled: bool) -> Result<AppConfig> {
    update(|c| c.hide_on_fullscreen = enabled)
}

#[tauri::command]
pub fn mark_onboarded() -> Result<AppConfig, String> {
    update(|c| c.onboarded = true).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_last_agent(character: String, path: Option<String>) -> Result<AppConfig, String> {
    update(|c| match character.as_str() {
        "bruce" => c.last_agent_bruce = path,
        "jazz" => c.last_agent_jazz = path,
        _ => {}
    })
    .map_err(|e| e.to_string())
}
