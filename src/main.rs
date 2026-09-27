//! Twilight PC — filtro de luz azul nativo para Linux (GTK 4 / libadwaita).
//!
//! Backend padrão: [sunsetr](https://github.com/psi4j/sunsetr), controlado via CLI.

mod app;
mod backend;
mod color;
mod config;
mod ui;

use app::AppModel;
use relm4::RelmApp;
use relm4::adw;
use tracing_subscriber::EnvFilter;

rust_i18n::i18n!("locales", fallback = "en");

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    rust_i18n::set_locale(&detect_locale());

    // Preferir AdwStyleManager em vez de GtkSettings:gtk-application-prefer-dark-theme.
    let _ = adw::init();
    apply_color_scheme();

    let app = RelmApp::new("org.twilight_pc.Twilight");
    apply_color_scheme();
    app.run::<AppModel>(());
}

fn apply_color_scheme() {
    // AdwStyleManager é a API suportada; evita o warning de
    // GtkSettings:gtk-application-prefer-dark-theme com libadwaita.
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::Default);
}

/// Idioma da interface a partir de LC_ALL/LANG ("pt_BR.UTF-8" → "pt").
fn detect_locale() -> String {
    let lang = std::env::var("LC_ALL")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var("LANG").ok())
        .unwrap_or_default()
        .to_lowercase();
    if lang.starts_with("pt") {
        "pt".into()
    } else {
        "en".into()
    }
}
