//! Cliente CLI para o daemon sunsetr (instância isolada do Twilight Linux).
//!
//! Usa `~/.config/.../sunsetr/` via `--config`, sem ler/escrever `~/.config/sunsetr`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use rust_i18n::t;
use serde::Deserialize;
use serde_json::Value;

const BIN: &str = "sunsetr";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendKind {
    #[default]
    Auto,
    Hyprland,
    Hyprsunset,
    Wayland,
}

impl BackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Hyprland => "hyprland",
            Self::Hyprsunset => "hyprsunset",
            Self::Wayland => "wayland",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "hyprland" => Self::Hyprland,
            "hyprsunset" => Self::Hyprsunset,
            "wayland" => Self::Wayland,
            _ => Self::Auto,
        }
    }

    /// Ferramenta/binário que o backend exige disponível no sistema.
    pub fn required_tool(self) -> Option<&'static str> {
        match self {
            Self::Hyprland => Some("hyprctl"),
            Self::Hyprsunset => Some("hyprsunset"),
            Self::Auto | Self::Wayland => None,
        }
    }

    /// Verifica se o backend escolhido pode funcionar neste sistema.
    pub fn is_supported(self) -> bool {
        match self {
            Self::Auto | Self::Wayland => true,
            Self::Hyprland => {
                std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
                    && tool_exists("hyprctl")
            }
            Self::Hyprsunset => {
                std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
                    && tool_exists("hyprsunset")
            }
        }
    }
}

fn tool_exists(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransitionMode {
    #[default]
    Geo,
    FinishBy,
    StartAt,
    Center,
    Static,
}

impl TransitionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Geo => "geo",
            Self::FinishBy => "finish_by",
            Self::StartAt => "start_at",
            Self::Center => "center",
            Self::Static => "static",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "finish_by" => Self::FinishBy,
            "start_at" => Self::StartAt,
            "center" => Self::Center,
            "static" => Self::Static,
            _ => Self::Geo,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Geo => t!("mode.geo"),
            Self::FinishBy => t!("mode.finish_by"),
            Self::StartAt => t!("mode.start_at"),
            Self::Center => t!("mode.center"),
            Self::Static => t!("mode.static"),
        }
        .into_owned()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Day,
    Sunset,
    Night,
    Sunrise,
    Static,
    Unknown,
}

impl Period {
    pub fn parse(s: &str) -> Self {
        match s {
            "day" => Self::Day,
            "sunset" => Self::Sunset,
            "night" => Self::Night,
            "sunrise" => Self::Sunrise,
            "static" => Self::Static,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Day => t!("period.day"),
            Self::Sunset => t!("period.sunset"),
            Self::Night => t!("period.night"),
            Self::Sunrise => t!("period.sunrise"),
            Self::Static => t!("period.static"),
            Self::Unknown => t!("period.unknown"),
        }
        .into_owned()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct RuntimeStatus {
    pub active_preset: Option<String>,
    pub period: String,
    pub state: Option<String>,
    pub progress: Option<f64>,
    pub current_temp: Option<u32>,
    pub current_gamma: Option<f64>,
    pub target_temp: Option<u32>,
    pub target_gamma: Option<f64>,
    pub next_period: Option<String>,
}

impl RuntimeStatus {
    pub fn period_kind(&self) -> Period {
        Period::parse(&self.period)
    }

    pub fn temp_kelvin(&self) -> u32 {
        self.current_temp
            .or(self.target_temp)
            .unwrap_or(6500)
            .clamp(1000, 20000)
    }

    pub fn gamma_percent(&self) -> f64 {
        self.current_gamma
            .or(self.target_gamma)
            .unwrap_or(100.0)
            .clamp(10.0, 200.0)
    }
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct SunsetrConfig {
    pub backend: BackendKind,
    pub transition_mode: TransitionMode,
    pub night_temp: u32,
    pub day_temp: u32,
    pub night_gamma: f64,
    pub day_gamma: f64,
    pub static_temp: u32,
    pub static_gamma: f64,
    pub sunset: String,
    pub sunrise: String,
    pub transition_duration: u32,
    pub latitude: f64,
    pub longitude: f64,
    pub smoothing: bool,
    pub startup_duration: f64,
    pub shutdown_duration: f64,
}

impl SunsetrConfig {
    fn from_map(map: &HashMap<String, String>) -> Self {
        let get = |k: &str| map.get(k).map(String::as_str).unwrap_or("");
        let parse_u32 = |k: &str, d: u32| get(k).parse().unwrap_or(d);
        let parse_f64 = |k: &str, d: f64| get(k).parse().unwrap_or(d);
        let parse_bool = |k: &str, d: bool| match get(k) {
            "true" => true,
            "false" => false,
            _ => d,
        };

        Self {
            backend: BackendKind::parse(get("backend")),
            transition_mode: TransitionMode::parse(get("transition_mode")),
            night_temp: parse_u32("night_temp", 3300),
            day_temp: parse_u32("day_temp", 6500),
            night_gamma: parse_f64("night_gamma", 90.0),
            day_gamma: parse_f64("day_gamma", 100.0),
            static_temp: parse_u32("static_temp", 3300),
            static_gamma: parse_f64("static_gamma", 90.0),
            sunset: {
                let s = get("sunset");
                if s.is_empty() {
                    "19:00:00".into()
                } else {
                    s.into()
                }
            },
            sunrise: {
                let s = get("sunrise");
                if s.is_empty() {
                    "06:00:00".into()
                } else {
                    s.into()
                }
            },
            transition_duration: parse_u32("transition_duration", 45),
            latitude: parse_f64("latitude", 0.0),
            longitude: parse_f64("longitude", 0.0),
            smoothing: parse_bool("smoothing", true),
            startup_duration: parse_f64("startup_duration", 0.5),
            shutdown_duration: parse_f64("shutdown_duration", 0.5),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SunsetrClient;

impl SunsetrClient {
    /// Diretório de config isolado do Twilight (nunca `~/.config/sunsetr`).
    pub fn config_dir() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("org", "twilightlinux", "TwilightLinux")
            .context("não foi possível resolver XDG config")?;
        Ok(dirs.config_dir().join("sunsetr"))
    }

    pub fn ensure_config_dir() -> Result<PathBuf> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)?;
        fs::create_dir_all(dir.join("presets"))?;
        let toml_path = dir.join("sunsetr.toml");
        if !toml_path.exists() {
            fs::write(&toml_path, DEFAULT_TOML).with_context(|| {
                format!("criar {}", toml_path.display())
            })?;
        }
        Ok(dir)
    }

    pub fn is_available() -> bool {
        Command::new(BIN)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn is_running() -> bool {
        Self::status().is_ok()
    }

    pub fn status() -> Result<RuntimeStatus> {
        let stdout = run_capture(&["status", "--json"], false)?;
        serde_json::from_str(&stdout).context("falha ao analisar status JSON do sunsetr")
    }

    pub fn get_config() -> Result<SunsetrConfig> {
        let stdout = run_capture(&["get", "all", "--json"], true)?;
        let value: Value =
            serde_json::from_str(&stdout).context("falha ao analisar config JSON do sunsetr")?;
        let obj = value
            .as_object()
            .ok_or_else(|| anyhow!("resposta get all não é um objeto JSON"))?;
        let mut map = HashMap::new();
        for (k, v) in obj {
            let s = match v {
                Value::String(s) => s.clone(),
                other => other.to_string().trim_matches('"').to_string(),
            };
            map.insert(k.clone(), s);
        }
        Ok(SunsetrConfig::from_map(&map))
    }

    pub fn set_fields(pairs: &[(&str, String)]) -> Result<()> {
        if pairs.is_empty() {
            return Ok(());
        }
        let mut args = Vec::new();
        for (k, v) in pairs {
            args.push(format!("{k}={v}"));
        }
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut full = vec!["set"];
        full.extend(arg_refs);
        run_status(&full, true)
    }

    pub fn set_current_temp_gamma(temp: u32, gamma: f64) -> Result<()> {
        Self::set_fields(&[
            ("current_temp", temp.to_string()),
            ("current_gamma", format_gamma(gamma)),
        ])
    }

    pub fn set_night_day(
        night_temp: u32,
        day_temp: u32,
        night_gamma: f64,
        day_gamma: f64,
    ) -> Result<()> {
        Self::set_fields(&[
            ("night_temp", night_temp.to_string()),
            ("day_temp", day_temp.to_string()),
            ("night_gamma", format_gamma(night_gamma)),
            ("day_gamma", format_gamma(day_gamma)),
        ])
    }

    pub fn set_transition_mode(mode: TransitionMode) -> Result<()> {
        Self::set_fields(&[("transition_mode", mode.as_str().into())])
    }

    pub fn set_schedule(sunset: &str, sunrise: &str, transition_minutes: u32) -> Result<()> {
        Self::set_fields(&[
            ("sunset", normalize_time(sunset)),
            ("sunrise", normalize_time(sunrise)),
            ("transition_duration", transition_minutes.to_string()),
        ])
    }

    pub fn set_location(lat: f64, lon: f64) -> Result<()> {
        Self::set_fields(&[
            ("latitude", format!("{lat:.6}")),
            ("longitude", format!("{lon:.6}")),
        ])
    }

    /// Define o backend do sunsetr e **reinicia a instância** para recriá-lo.
    ///
    /// Trocar `hyprland` / `hyprsunset` / `wayland` só tem efeito depois de
    /// recriar o backend do daemon — por isso o restart quando está rodando.
    pub fn set_backend(kind: BackendKind) -> Result<()> {
        Self::set_fields(&[("backend", kind.as_str().into())])?;
        if Self::is_running() {
            Self::restart()?;
        }
        Ok(())
    }

    /// Para a instância isolada e inicia de novo com a config atual.
    pub fn restart() -> Result<()> {
        let dir = Self::ensure_config_dir()?;
        Self::stop_any()?;
        start_detached(&dir)?;
        if wait_until_running(Duration::from_secs(4)) {
            return Ok(());
        }
        bail!(
            "{}",
            t!("error.start_sunsetr", dir = dir.display().to_string())
        )
    }

    pub fn set_smoothing(startup: f64, shutdown: f64, enabled: bool) -> Result<()> {
        Self::set_fields(&[
            ("smoothing", enabled.to_string()),
            ("startup_duration", format!("{startup:.2}")),
            ("shutdown_duration", format!("{shutdown:.2}")),
        ])
    }

    pub fn ensure_running() -> Result<()> {
        let dir = Self::ensure_config_dir()?;

        if Self::is_running() {
            // Já há uma instância; se não for a nossa, paramos e reiniciamos.
            if instance_uses_dir(&dir) {
                return Ok(());
            }
            let _ = Self::stop_any();
        }

        // Para o sunsetr padrão / systemd do usuário para não brigar no CTM.
        let _ = Self::stop_any();

        start_detached(&dir)?;
        if wait_until_running(Duration::from_secs(4)) {
            return Ok(());
        }

        bail!(
            "{}",
            t!("error.start_sunsetr", dir = dir.display().to_string())
        )
    }

    pub fn stop() -> Result<()> {
        Self::stop_any()
    }

    fn stop_any() -> Result<()> {
        // Para qualquer instância ativa (padrão ou Twilight).
        let _ = Command::new(BIN)
            .arg("stop")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("systemctl")
            .args(["--user", "stop", "sunsetr.service"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("systemctl")
            .args(["--user", "stop", "twilight-sunsetr.service"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        // Aguarda o lock liberar.
        for _ in 0..20 {
            if !Self::is_running() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    pub fn apply_ui_preset(preset: UiPreset, gamma: f64) -> Result<()> {
        match preset {
            UiPreset::Day => {
                Self::set_fields(&[
                    ("transition_mode", "static".into()),
                    ("static_temp", "6500".into()),
                    ("static_gamma", format_gamma(gamma.max(100.0))),
                ])?;
            }
            UiPreset::Sunset => {
                Self::set_fields(&[
                    ("transition_mode", "static".into()),
                    ("static_temp", "4500".into()),
                    ("static_gamma", format_gamma(gamma)),
                ])?;
            }
            UiPreset::Night => {
                Self::set_fields(&[
                    ("transition_mode", "static".into()),
                    ("static_temp", "3300".into()),
                    ("static_gamma", format_gamma(gamma.min(90.0))),
                ])?;
            }
            UiPreset::Custom => {}
            UiPreset::Schedule => {
                let cfg = Self::get_config()?;
                let mode = if cfg.transition_mode == TransitionMode::Static {
                    TransitionMode::Geo
                } else {
                    cfg.transition_mode
                };
                Self::set_transition_mode(mode)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPreset {
    Day,
    Sunset,
    Night,
    Custom,
    Schedule,
}

impl UiPreset {
    /// Identificador estável para persistência (independe do idioma).
    pub fn key(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Sunset => "sunset",
            Self::Night => "night",
            Self::Custom => "custom",
            Self::Schedule => "schedule",
        }
    }

    pub fn from_key(s: &str) -> Self {
        match s {
            "day" => Self::Day,
            "sunset" => Self::Sunset,
            "night" => Self::Night,
            "custom" => Self::Custom,
            _ => Self::Schedule,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Day => t!("preset.day"),
            Self::Sunset => t!("preset.sunset"),
            Self::Night => t!("preset.night"),
            Self::Custom => t!("preset.custom"),
            Self::Schedule => t!("preset.schedule"),
        }
        .into_owned()
    }
}

const DEFAULT_TOML: &str = r#"# Twilight Linux — config isolada (não usa ~/.config/sunsetr)
backend = "auto"
transition_mode = "static"
smoothing = true
startup_duration = 0.5
shutdown_duration = 0.5
adaptive_interval = 1
night_temp = 3300
day_temp = 6500
night_gamma = 90
day_gamma = 100
update_interval = "auto"
static_temp = 3300
static_gamma = 90
sunset = "19:00:00"
sunrise = "06:00:00"
transition_duration = 45
"#;

fn format_gamma(g: f64) -> String {
    if (g - g.round()).abs() < f64::EPSILON {
        format!("{}", g.round() as i32)
    } else {
        format!("{g:.1}")
    }
}

fn normalize_time(t: &str) -> String {
    let t = t.trim();
    if t.len() == 5 && t.chars().nth(2) == Some(':') {
        format!("{t}:00")
    } else {
        t.to_string()
    }
}

fn config_args(with_config: bool) -> Result<Vec<String>> {
    let mut args = Vec::new();
    if with_config {
        let dir = SunsetrClient::ensure_config_dir()?;
        args.push("--config".into());
        args.push(dir.to_string_lossy().into_owned());
    }
    Ok(args)
}

fn run_capture(cmd_args: &[&str], with_config: bool) -> Result<String> {
    let mut args = config_args(with_config)?;
    args.extend(cmd_args.iter().map(|s| (*s).to_string()));
    let output = Command::new(BIN)
        .args(&args)
        .output()
        .with_context(|| format!("falha ao executar {BIN}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{BIN} falhou: {stderr}");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_status(cmd_args: &[&str], with_config: bool) -> Result<()> {
    let mut args = config_args(with_config)?;
    args.extend(cmd_args.iter().map(|s| (*s).to_string()));
    let output = Command::new(BIN)
        .args(&args)
        .output()
        .with_context(|| format!("falha ao executar {BIN}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };
        bail!("{BIN} falhou: {msg}");
    }
    Ok(())
}

fn start_detached(dir: &Path) -> Result<()> {
    let mut child = Command::new(BIN)
        .arg("--config")
        .arg(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("falha ao lançar sunsetr em processo separado")?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn wait_until_running(timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if SunsetrClient::is_running() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// Verifica se o processo sunsetr ativo foi iniciado com o nosso `--config`.
fn instance_uses_dir(dir: &Path) -> bool {
    let Ok(output) = Command::new("pgrep").args(["-a", "-f", "sunsetr"]).output() else {
        return false;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let dir_s = dir.to_string_lossy();
    text.lines().any(|line| {
        line.contains("sunsetr")
            && line.contains("--config")
            && line.contains(dir_s.as_ref())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_hhmm() {
        assert_eq!(normalize_time("19:30"), "19:30:00");
        assert_eq!(normalize_time("06:00:00"), "06:00:00");
    }

    #[test]
    fn period_labels() {
        rust_i18n::set_locale("pt");
        assert_eq!(Period::parse("night").label(), "Noite");
        assert_eq!(TransitionMode::parse("geo").as_str(), "geo");
        assert_eq!(UiPreset::from_key("night"), UiPreset::Night);
        rust_i18n::set_locale("en");
        assert_eq!(Period::parse("night").label(), "Night");
        rust_i18n::set_locale("pt");
    }

    #[test]
    fn config_from_map_defaults() {
        let map = HashMap::from([
            ("night_temp".into(), "3000".into()),
            ("backend".into(), "hyprland".into()),
        ]);
        let cfg = SunsetrConfig::from_map(&map);
        assert_eq!(cfg.night_temp, 3000);
        assert_eq!(cfg.backend, BackendKind::Hyprland);
        assert_eq!(cfg.day_temp, 6500);
    }
}
