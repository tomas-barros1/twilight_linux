//! Janela principal Adwaita com navegação Filtro / Ajustes.

use std::time::{Duration, Instant};

use relm4::adw::prelude::*;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
    adw, gtk, gtk::glib,
};

use crate::backend::{
    BackendKind, ConflictReport, Period, SunsetrClient, SunsetrConfig, TransitionMode, UiPreset,
    detect_conflicts,
};
use crate::color::{intensity_to_kelvin, kelvin_to_intensity};
use crate::config::{self, AppConfig};
use crate::ui::dashboard::{
    DashboardInit, DashboardInput, DashboardModel, DashboardOutput,
};
use crate::ui::settings::{SettingsInit, SettingsInput, SettingsModel, SettingsOutput};

#[derive(Debug)]
pub enum AppMsg {
    Dashboard(DashboardOutput),
    Settings(SettingsOutput),
    Refresh,
    StatusLoaded {
        running: bool,
        status: Option<crate::backend::RuntimeStatus>,
        cfg: Option<SunsetrConfig>,
        error: Option<String>,
        conflicts: Vec<ConflictReport>,
    },
    BackendDone {
        error: Option<String>,
        refresh: bool,
    },
    DebouncedApply,
}

pub struct AppModel {
    app_config: AppConfig,
    enabled: bool,
    temperature: u32,
    intensity: u32,
    gamma: f64,
    period: Period,
    preset: UiPreset,
    next_period: String,
    backend_ok: bool,
    error: Option<String>,
    conflicts: Vec<ConflictReport>,
    sunsetr_cfg: SunsetrConfig,
    dashboard: Controller<DashboardModel>,
    settings: Controller<SettingsModel>,
    last_slider_change: Option<Instant>,
    pending_apply: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        #[root]
        adw::ApplicationWindow {
            set_title: Some("Twilight"),
            set_default_width: 520,
            set_default_height: 720,
            #[watch]
            set_hide_on_close: model.app_config.hide_on_close,

            #[wrap(Some)]
            set_content = &adw::ToastOverlay {
                #[name(toast_overlay)]

                #[wrap(Some)]
                set_child = &adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {
                        #[wrap(Some)]
                        set_title_widget = &adw::WindowTitle {
                            set_title: "Twilight",
                            set_subtitle: "Filtro de luz azul",
                        },
                    },

                    #[wrap(Some)]
                    set_content = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        #[name(stack)]
                        adw::ViewStack {
                            set_vexpand: true,
                            add_titled_with_icon: (
                                model.dashboard.widget(),
                                Some("filter"),
                                "Filtro",
                                "night-light-symbolic",
                            ),
                            add_titled_with_icon: (
                                model.settings.widget(),
                                Some("settings"),
                                "Ajustes",
                                "emblem-system-symbolic",
                            ),
                        },

                        adw::ViewSwitcherBar {
                            set_stack: Some(&stack),
                            set_reveal: true,
                        },
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let app_config = AppConfig::load();
        let backend_ok = SunsetrClient::is_available();
        let enabled = backend_ok && SunsetrClient::is_running();

        let temperature = app_config.preferred_temp;
        let intensity = app_config.intensity;
        let gamma = app_config.preferred_gamma;
        let preset = app_config.ui_preset();

        let dashboard = DashboardModel::builder()
            .launch(DashboardInit {
                enabled,
                temperature,
                intensity,
                gamma,
                period: Period::Unknown,
                preset,
                preset_label: preset.label().to_string(),
                next_period: String::new(),
                backend_ok,
                error: if backend_ok {
                    None
                } else {
                    Some("sunsetr não está instalado".into())
                },
                conflicts: Vec::new(),
            })
            .forward(sender.input_sender(), AppMsg::Dashboard);

        let settings = SettingsModel::builder()
            .launch(SettingsInit {
                transition_mode: TransitionMode::Geo,
                sunset: "19:00:00".into(),
                sunrise: "06:00:00".into(),
                transition_duration: 45,
                latitude: 0.0,
                longitude: 0.0,
                backend: BackendKind::Auto,
                start_on_login: config::systemd_autostart_enabled(),
                hide_on_close: app_config.hide_on_close,
                day_temp: 6500,
                night_temp: 3300,
                day_gamma: 100.0,
                night_gamma: 90.0,
                startup_duration: 0.5,
                conflicts: Vec::new(),
                backend_ok,
            })
            .forward(sender.input_sender(), AppMsg::Settings);

        let model = AppModel {
            app_config,
            enabled,
            temperature,
            intensity,
            gamma,
            period: Period::Unknown,
            preset,
            next_period: String::new(),
            backend_ok,
            error: None,
            conflicts: Vec::new(),
            sunsetr_cfg: SunsetrConfig::default(),
            dashboard,
            settings,
            last_slider_change: None,
            pending_apply: false,
        };

        let widgets = view_output!();

        sender.input(AppMsg::Refresh);
        glib::timeout_add_local(Duration::from_secs(5), {
            let sender = sender.clone();
            move || {
                sender.input(AppMsg::Refresh);
                glib::ControlFlow::Continue
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.update_inner(msg, sender);
    }
}

impl AppModel {
    fn update_inner(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Refresh => {
                spawn_refresh(sender.clone());
            }
            AppMsg::StatusLoaded {
                running,
                status,
                cfg,
                error,
                conflicts,
            } => {
                // Não sobrescrever sliders durante arraste / debounce.
                if self.pending_apply {
                    self.enabled = running;
                    self.error = error;
                    self.conflicts = conflicts;
                    if let Some(cfg) = cfg {
                        self.sunsetr_cfg = cfg;
                    }
                    return;
                }
                self.enabled = running;
                self.error = error;
                self.conflicts = conflicts;
                if let Some(cfg) = cfg {
                    self.sunsetr_cfg = cfg;
                }
                if let Some(st) = status {
                    self.period = st.period_kind();
                    self.temperature = st.temp_kelvin().min(6500);
                    self.gamma = st.gamma_percent();
                    self.intensity = kelvin_to_intensity(self.temperature);
                    self.next_period = st.next_period.clone().unwrap_or_default();
                }
                self.sync_children();
            }
            AppMsg::Dashboard(out) => self.handle_dashboard(out, sender),
            AppMsg::Settings(out) => self.handle_settings(out, sender),
            AppMsg::DebouncedApply => {
                if let Some(t) = self.last_slider_change {
                    if t.elapsed() < Duration::from_millis(180) {
                        return;
                    }
                }
                if !self.pending_apply || !self.enabled {
                    return;
                }
                self.pending_apply = false;
                let temp = self.temperature;
                let gamma = self.gamma;
                self.app_config.preferred_temp = temp;
                self.app_config.preferred_gamma = gamma;
                self.app_config.intensity = self.intensity;
                self.app_config.last_ui_preset = UiPreset::Custom.label().into();
                self.preset = UiPreset::Custom;
                let _ = self.app_config.save();
                spawn_backend(sender, move || {
                    // Só aplica se já estiver rodando; o toggle é quem inicia.
                    if !SunsetrClient::is_running() {
                        anyhow::bail!("filtro desligado — ligue o interruptor primeiro");
                    }
                    SunsetrClient::set_current_temp_gamma(temp, gamma)?;
                    Ok(())
                });
            }
            AppMsg::BackendDone { error, refresh } => {
                if let Some(err) = error {
                    self.error = Some(err.clone());
                    // Se falhou ao ligar/iniciar, reverte o interruptor.
                    if err.contains("iniciar o sunsetr") || err.contains("lançar sunsetr") {
                        self.enabled = false;
                        self.pending_apply = false;
                    }
                    self.sync_children();
                } else {
                    self.error = None;
                    if refresh {
                        sender.input(AppMsg::Refresh);
                    }
                }
            }
        }
    }

    fn sync_children(&self) {
        let conflict_strs: Vec<String> = self
            .conflicts
            .iter()
            .map(|c| format!("{} — {}", c.name, c.detail))
            .collect();

        self.dashboard.emit(DashboardInput::Sync(DashboardInit {
            enabled: self.enabled,
            temperature: self.temperature,
            intensity: self.intensity,
            gamma: self.gamma,
            period: self.period,
            preset: self.preset,
            preset_label: self.preset.label().to_string(),
            next_period: self.next_period.clone(),
            backend_ok: self.backend_ok,
            error: self.error.clone(),
            conflicts: conflict_strs.clone(),
        }));

        let cfg = &self.sunsetr_cfg;
        self.settings.emit(SettingsInput::Sync(SettingsInit {
            transition_mode: cfg.transition_mode,
            sunset: cfg.sunset.clone(),
            sunrise: cfg.sunrise.clone(),
            transition_duration: cfg.transition_duration,
            latitude: cfg.latitude,
            longitude: cfg.longitude,
            backend: cfg.backend,
            start_on_login: config::systemd_autostart_enabled(),
            hide_on_close: self.app_config.hide_on_close,
            day_temp: cfg.day_temp,
            night_temp: cfg.night_temp,
            day_gamma: cfg.day_gamma,
            night_gamma: cfg.night_gamma,
            startup_duration: cfg.startup_duration,
            conflicts: conflict_strs,
            backend_ok: self.backend_ok,
        }));
    }

    fn schedule_apply(&mut self, sender: &ComponentSender<Self>) {
        self.last_slider_change = Some(Instant::now());
        self.pending_apply = true;
        let sender = sender.clone();
        glib::timeout_add_local_once(Duration::from_millis(200), move || {
            sender.input(AppMsg::DebouncedApply);
        });
    }

    fn handle_dashboard(&mut self, out: DashboardOutput, sender: ComponentSender<Self>) {
        match out {
            DashboardOutput::SetEnabled(on) => {
                if on {
                    spawn_backend(sender.clone(), || {
                        SunsetrClient::ensure_running()?;
                        Ok(())
                    });
                    self.enabled = true;
                    self.error = None;
                } else {
                    self.pending_apply = false;
                    spawn_backend(sender.clone(), || {
                        SunsetrClient::stop()?;
                        Ok(())
                    });
                    self.enabled = false;
                }
                self.sync_children();
            }
            DashboardOutput::SetTemperature(temp) => {
                self.temperature = temp;
                self.intensity = kelvin_to_intensity(temp);
                self.schedule_apply(&sender);
            }
            DashboardOutput::SetIntensity(intensity) => {
                self.intensity = intensity;
                self.temperature = intensity_to_kelvin(intensity);
                self.schedule_apply(&sender);
            }
            DashboardOutput::SetGamma(gamma) => {
                self.gamma = gamma;
                self.schedule_apply(&sender);
            }
            DashboardOutput::ApplyPreset(preset) => {
                self.preset = preset;
                self.app_config.last_ui_preset = preset.label().into();
                let _ = self.app_config.save();
                let gamma = self.gamma;
                match preset {
                    UiPreset::Day => {
                        self.temperature = 6500;
                        self.intensity = 0;
                        self.gamma = gamma.max(100.0);
                    }
                    UiPreset::Sunset => {
                        self.temperature = 4500;
                        self.intensity = kelvin_to_intensity(4500);
                    }
                    UiPreset::Night => {
                        self.temperature = 3300;
                        self.intensity = kelvin_to_intensity(3300);
                        self.gamma = gamma.min(90.0);
                    }
                    UiPreset::Custom | UiPreset::Schedule => {}
                }
                spawn_backend(sender, move || {
                    if !SunsetrClient::is_running() {
                        SunsetrClient::ensure_running()?;
                    }
                    SunsetrClient::apply_ui_preset(preset, gamma)?;
                    Ok(())
                });
                self.sync_children();
            }
            DashboardOutput::RequestRefresh => sender.input(AppMsg::Refresh),
        }
    }

    fn handle_settings(&mut self, out: SettingsOutput, sender: ComponentSender<Self>) {
        match out {
            SettingsOutput::SetTransitionMode(mode) => {
                spawn_backend(sender, move || {
                    SunsetrClient::set_transition_mode(mode)?;
                    Ok(())
                });
            }
            SettingsOutput::SetBackend(kind) => {
                spawn_backend(sender, move || {
                    SunsetrClient::set_backend(kind)?;
                    Ok(())
                });
            }
            SettingsOutput::SetSchedule {
                sunset,
                sunrise,
                transition_minutes,
            } => {
                spawn_backend(sender, move || {
                    SunsetrClient::set_schedule(&sunset, &sunrise, transition_minutes)?;
                    Ok(())
                });
            }
            SettingsOutput::SetLocation { lat, lon } => {
                spawn_backend(sender, move || {
                    SunsetrClient::set_location(lat, lon)?;
                    Ok(())
                });
            }
            SettingsOutput::SetDayNight {
                day_temp,
                night_temp,
                day_gamma,
                night_gamma,
            } => {
                spawn_backend(sender, move || {
                    SunsetrClient::set_night_day(night_temp, day_temp, night_gamma, day_gamma)?;
                    Ok(())
                });
            }
            SettingsOutput::SetSmoothing { startup } => {
                spawn_backend(sender, move || {
                    SunsetrClient::set_smoothing(startup, startup, true)?;
                    Ok(())
                });
            }
            SettingsOutput::SetStartOnLogin(on) => {
                self.app_config.start_filter_on_login = on;
                let _ = self.app_config.save();
                spawn_backend(sender, move || {
                    config::set_systemd_autostart(on)?;
                    Ok(())
                });
            }
            SettingsOutput::SetHideOnClose(on) => {
                self.app_config.hide_on_close = on;
                let _ = self.app_config.save();
            }
        }
    }
}

fn spawn_refresh(sender: ComponentSender<AppModel>) {
    std::thread::spawn(move || {
        let backend_ok = SunsetrClient::is_available();
        if !backend_ok {
            sender.input(AppMsg::StatusLoaded {
                running: false,
                status: None,
                cfg: None,
                error: Some("sunsetr não encontrado no PATH".into()),
                conflicts: Vec::new(),
            });
            return;
        }

        let running = SunsetrClient::is_running();
        let status = if running {
            SunsetrClient::status().ok()
        } else {
            None
        };
        let cfg = SunsetrClient::get_config().ok();
        let conflicts = detect_conflicts(running);

        sender.input(AppMsg::StatusLoaded {
            running,
            status,
            cfg,
            error: None,
            conflicts,
        });
    });
}

fn spawn_backend<F>(sender: ComponentSender<AppModel>, f: F)
where
    F: FnOnce() -> anyhow::Result<()> + Send + 'static,
{
    std::thread::spawn(move || {
        let error = f().err().map(|e| e.to_string());
        sender.input(AppMsg::BackendDone {
            error,
            refresh: true,
        });
    });
}
