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
use relm4::{adw, gtk};
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Preferir AdwStyleManager em vez de GtkSettings:gtk-application-prefer-dark-theme.
    let _ = adw::init();
    apply_color_scheme();

    let app = RelmApp::new("org.twilight_pc.Twilight");
    apply_color_scheme();
    app.run::<AppModel>(());
}

fn apply_color_scheme() {
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::Default);
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(false);
    }
}
