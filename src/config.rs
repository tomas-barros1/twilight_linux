//! Configuração persistente do Twilight Linux (não confundir com sunsetr.toml).

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::backend::UiPreset;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Preferência de iniciar o filtro com a sessão (via systemd --user sunsetr).
    #[serde(default)]
    pub start_filter_on_login: bool,

    /// Esconder a janela ao fechar (filtro continua via sunsetr).
    #[serde(default = "default_true")]
    pub hide_on_close: bool,

    /// Último preset de UI selecionado.
    #[serde(default)]
    pub last_ui_preset: String,

    /// Temperatura preferida para o modo personalizado (K).
    #[serde(default = "default_temp")]
    pub preferred_temp: u32,

    /// Gama preferida (%).
    #[serde(default = "default_gamma")]
    pub preferred_gamma: f64,

    /// Intensidade 0–100 (ligada à temperatura).
    #[serde(default = "default_intensity")]
    pub intensity: u32,
}

fn default_true() -> bool {
    true
}
fn default_temp() -> u32 {
    3300
}
fn default_gamma() -> f64 {
    90.0
}
fn default_intensity() -> u32 {
    84
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            start_filter_on_login: false,
            hide_on_close: true,
            last_ui_preset: UiPreset::Schedule.key().to_string(),
            preferred_temp: default_temp(),
            preferred_gamma: default_gamma(),
            intensity: default_intensity(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        match Self::path() {
            Ok(path) if path.exists() => fs::read_to_string(&path)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
                .unwrap_or_default(),
            _ => Self::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = toml::to_string_pretty(self).context("serializar config")?;
        fs::write(&path, data).with_context(|| format!("gravar {}", path.display()))?;
        Ok(())
    }

    pub fn path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("org", "twilightlinux", "TwilightLinux")
            .context("não foi possível resolver XDG config")?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    pub fn ui_preset(&self) -> UiPreset {
        // Aceita também os rótulos antigos em pt-BR (compatibilidade).
        match self.last_ui_preset.as_str() {
            "Dia" => UiPreset::Day,
            "Pôr do sol" => UiPreset::Sunset,
            "Noite" => UiPreset::Night,
            "Personalizado" => UiPreset::Custom,
            other => UiPreset::from_key(other),
        }
    }
}

/// Liga/desliga autostart do filtro Twilight (sunsetr com `--config` isolado).
pub fn set_systemd_autostart(enable: bool) -> Result<()> {
    let unit_path = systemd_unit_path()?;
    if enable {
        let unit = unit_contents()?;
        if let Some(parent) = unit_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&unit_path, unit)?;
        let status = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()
            .context("systemctl daemon-reload")?;
        if !status.success() {
            anyhow::bail!("systemctl daemon-reload falhou");
        }
        // Desativa o sunsetr padrão para não conflitar.
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "disable", "--now", "sunsetr.service"])
            .status();
        let status = std::process::Command::new("systemctl")
            .args(["--user", "enable", "--now", "twilight-sunsetr.service"])
            .status()
            .context("systemctl enable twilight-sunsetr")?;
        if !status.success() {
            anyhow::bail!("falha ao ativar twilight-sunsetr.service");
        }
    } else {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "disable", "--now", "twilight-sunsetr.service"])
            .status();
    }
    Ok(())
}

pub fn systemd_autostart_enabled() -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", "twilight-sunsetr.service"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "enabled")
        .unwrap_or(false)
}

fn systemd_unit_path() -> Result<PathBuf> {
    let home = directories::BaseDirs::new().context("home")?;
    Ok(home
        .home_dir()
        .join(".config/systemd/user/twilight-sunsetr.service"))
}

/// Conteúdo da unit `twilight-sunsetr.service` (config isolada do app).
fn unit_contents() -> Result<String> {
    let config_dir = crate::backend::SunsetrClient::ensure_config_dir()?;
    Ok(format!(
        r#"[Unit]
Description=Twilight Linux blue-light filter (sunsetr isolado)
PartOf=graphical-session.target
After=graphical-session.target
ConditionEnvironment=WAYLAND_DISPLAY

[Service]
Type=simple
ExecStart=/usr/bin/sunsetr --config {config}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=graphical-session.target
"#,
        config = config_dir.display()
    ))
}

/// Migra a configuração da identidade antiga (`twilight_pc` → `twilight_linux`).
///
/// Move `~/.config/twilight` → `~/.config/twilightlinux` e regrava a unit
/// `twilight-sunsetr.service` se ela ainda apontar para o caminho legado.
/// Retorna `true` quando a pasta foi movida.
pub fn migrate_legacy_config() -> Result<bool> {
    let moved = move_legacy_dir()?;
    refresh_systemd_unit_if_stale()?;
    Ok(moved)
}

fn move_legacy_dir() -> Result<bool> {
    let legacy = legacy_config_dir()?;
    let current = AppConfig::path()?
        .parent()
        .context("pai do config dir")?
        .to_path_buf();

    if !legacy.exists() || current.exists() {
        return Ok(false);
    }
    if let Some(parent) = current.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&legacy, &current)
        .with_context(|| format!("migrar {} → {}", legacy.display(), current.display()))?;
    tracing::info!(
        "config migrada: {} → {}",
        legacy.display(),
        current.display()
    );
    Ok(true)
}

/// Caminho legado usado pelo `twilight_pc` (~/.config/twilight).
fn legacy_config_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("org", "twilight_pc", "Twilight")
        .context("não foi possível resolver XDG config")?;
    Ok(dirs.config_dir().to_path_buf())
}

/// Se a unit do usuário ainda aponta para `~/.config/twilight/`, regrava com o
/// caminho novo, recarrega o systemd e reinicia o serviço se estava ativo.
fn refresh_systemd_unit_if_stale() -> Result<()> {
    let unit_path = systemd_unit_path()?;
    let Ok(content) = fs::read_to_string(&unit_path) else {
        return Ok(());
    };
    if !content.contains(".config/twilight/") {
        return Ok(());
    }

    let fresh = unit_contents()?;
    fs::write(&unit_path, fresh)?;
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    let active = std::process::Command::new("systemctl")
        .args(["--user", "is-active", "twilight-sunsetr.service"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if active {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "restart", "twilight-sunsetr.service"])
            .status();
    }
    tracing::info!("twilight-sunsetr.service regravada com o config dir novo");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_roundtrip_toml() {
        let c = AppConfig::default();
        let s = toml::to_string(&c).unwrap();
        let back: AppConfig = toml::from_str(&s).unwrap();
        assert_eq!(back.preferred_temp, 3300);
        assert!(back.hide_on_close);
    }
}
