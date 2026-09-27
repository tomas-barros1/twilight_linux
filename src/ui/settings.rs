//! Página de configurações: agendamento, localização, backend e inicialização.

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk, gtk::glib};
use rust_i18n::t;

use crate::backend::{BackendKind, TransitionMode};

#[derive(Debug, Clone)]
pub struct SettingsInit {
    pub transition_mode: TransitionMode,
    pub sunset: String,
    pub sunrise: String,
    pub transition_duration: u32,
    pub latitude: f64,
    pub longitude: f64,
    pub backend: BackendKind,
    pub start_on_login: bool,
    pub hide_on_close: bool,
    pub day_temp: u32,
    pub night_temp: u32,
    pub day_gamma: f64,
    pub night_gamma: f64,
    pub startup_duration: f64,
    pub conflicts: Vec<String>,
    pub backend_ok: bool,
}

#[derive(Debug)]
pub enum SettingsInput {
    Sync(SettingsInit),
    ModeChanged(u32),
    BackendChanged(u32),
    SunsetChanged(String),
    SunriseChanged(String),
    TransitionChanged(u32),
    LatChanged(f64),
    LonChanged(f64),
    DayTempChanged(u32),
    NightTempChanged(u32),
    DayGammaChanged(f64),
    NightGammaChanged(f64),
    StartupDurationChanged(f64),
    StartOnLogin(bool),
    HideOnClose(bool),
    ApplySchedule,
    ApplyLocation,
    ApplyTemps,
}

#[derive(Debug)]
pub enum SettingsOutput {
    SetTransitionMode(TransitionMode),
    SetBackend(BackendKind),
    SetSchedule {
        sunset: String,
        sunrise: String,
        transition_minutes: u32,
    },
    SetLocation {
        lat: f64,
        lon: f64,
    },
    SetDayNight {
        day_temp: u32,
        night_temp: u32,
        day_gamma: f64,
        night_gamma: f64,
    },
    SetSmoothing {
        startup: f64,
    },
    SetStartOnLogin(bool),
    SetHideOnClose(bool),
}

pub struct SettingsModel {
    pub data: SettingsInit,
    suppressing: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for SettingsModel {
    type Init = SettingsInit;
    type Input = SettingsInput;
    type Output = SettingsOutput;

    view! {
        adw::PreferencesPage {
            set_title: &t!("tab.settings"),

            add = &adw::PreferencesGroup {
                set_title: &t!("settings.schedule"),
                set_description: Some(&t!("settings.schedule.desc")),
                #[watch]
                set_sensitive: model.data.backend_ok,

                adw::ComboRow {
                    set_title: &t!("settings.mode"),
                    set_model: Some(&mode_list()),
                    #[watch]
                    #[block_signal(mode_handler)]
                    set_selected: mode_index(model.data.transition_mode),
                    connect_selected_notify[sender] => move |row| {
                        sender.input(SettingsInput::ModeChanged(row.selected()));
                    } @mode_handler,
                },

                adw::EntryRow {
                    set_title: &t!("settings.sunset"),
                    #[watch]
                    #[block_signal(sunset_handler)]
                    set_text: &hhmm(&model.data.sunset),
                    connect_changed[sender] => move |row| {
                        sender.input(SettingsInput::SunsetChanged(row.text().to_string()));
                    } @sunset_handler,
                },

                adw::EntryRow {
                    set_title: &t!("settings.sunrise"),
                    #[watch]
                    #[block_signal(sunrise_handler)]
                    set_text: &hhmm(&model.data.sunrise),
                    connect_changed[sender] => move |row| {
                        sender.input(SettingsInput::SunriseChanged(row.text().to_string()));
                    } @sunrise_handler,
                },

                adw::SpinRow {
                    set_title: &t!("settings.transition"),
                    set_subtitle: &t!("settings.transition.sub"),
                    set_adjustment: Some(&gtk::Adjustment::new(
                        model.data.transition_duration as f64,
                        1.0,
                        180.0,
                        1.0,
                        5.0,
                        0.0,
                    )),
                    set_digits: 0,
                    #[watch]
                    #[block_signal(trans_handler)]
                    set_value: model.data.transition_duration as f64,
                    connect_changed[sender] => move |row| {
                        sender.input(SettingsInput::TransitionChanged(row.value() as u32));
                    } @trans_handler,
                },

                adw::ActionRow {
                    set_title: &t!("settings.apply_schedule"),
                    add_suffix = &gtk::Button {
                        set_label: &t!("action.apply"),
                        set_valign: gtk::Align::Center,
                        add_css_class: "suggested-action",
                        connect_clicked[sender] => move |_| {
                            sender.input(SettingsInput::ApplySchedule);
                        },
                    },
                },
            },

            add = &adw::PreferencesGroup {
                set_title: &t!("settings.temps"),
                #[watch]
                set_sensitive: model.data.backend_ok,

                adw::SpinRow {
                    set_title: &t!("settings.day_temp"),
                    set_adjustment: Some(&gtk::Adjustment::new(
                        model.data.day_temp as f64, 1000.0, 20000.0, 50.0, 200.0, 0.0,
                    )),
                    set_digits: 0,
                    #[watch]
                    #[block_signal(day_t_handler)]
                    set_value: model.data.day_temp as f64,
                    connect_changed[sender] => move |row| {
                        sender.input(SettingsInput::DayTempChanged(row.value() as u32));
                    } @day_t_handler,
                },

                adw::SpinRow {
                    set_title: &t!("settings.night_temp"),
                    set_adjustment: Some(&gtk::Adjustment::new(
                        model.data.night_temp as f64, 1000.0, 20000.0, 50.0, 200.0, 0.0,
                    )),
                    set_digits: 0,
                    #[watch]
                    #[block_signal(night_t_handler)]
                    set_value: model.data.night_temp as f64,
                    connect_changed[sender] => move |row| {
                        sender.input(SettingsInput::NightTempChanged(row.value() as u32));
                    } @night_t_handler,
                },

                adw::SpinRow {
                    set_title: &t!("settings.day_gamma"),
                    set_adjustment: Some(&gtk::Adjustment::new(
                        model.data.day_gamma, 10.0, 200.0, 1.0, 5.0, 0.0,
                    )),
                    set_digits: 0,
                    #[watch]
                    #[block_signal(day_g_handler)]
                    set_value: model.data.day_gamma,
                    connect_changed[sender] => move |row| {
                        sender.input(SettingsInput::DayGammaChanged(row.value()));
                    } @day_g_handler,
                },

                adw::SpinRow {
                    set_title: &t!("settings.night_gamma"),
                    set_adjustment: Some(&gtk::Adjustment::new(
                        model.data.night_gamma, 10.0, 200.0, 1.0, 5.0, 0.0,
                    )),
                    set_digits: 0,
                    #[watch]
                    #[block_signal(night_g_handler)]
                    set_value: model.data.night_gamma,
                    connect_changed[sender] => move |row| {
                        sender.input(SettingsInput::NightGammaChanged(row.value()));
                    } @night_g_handler,
                },

                adw::ActionRow {
                    set_title: &t!("settings.apply_temps"),
                    add_suffix = &gtk::Button {
                        set_label: &t!("action.apply"),
                        set_valign: gtk::Align::Center,
                        add_css_class: "suggested-action",
                        connect_clicked[sender] => move |_| {
                            sender.input(SettingsInput::ApplyTemps);
                        },
                    },
                },
            },

            add = &adw::PreferencesGroup {
                set_title: &t!("settings.location"),
                set_description: Some(&t!("settings.location.desc")),
                #[watch]
                set_sensitive: model.data.backend_ok,

                adw::SpinRow {
                    set_title: &t!("settings.latitude"),
                    set_adjustment: Some(&gtk::Adjustment::new(
                        model.data.latitude, -90.0, 90.0, 0.01, 1.0, 0.0,
                    )),
                    set_digits: 4,
                    #[watch]
                    #[block_signal(lat_handler)]
                    set_value: model.data.latitude,
                    connect_changed[sender] => move |row| {
                        sender.input(SettingsInput::LatChanged(row.value()));
                    } @lat_handler,
                },

                adw::SpinRow {
                    set_title: &t!("settings.longitude"),
                    set_adjustment: Some(&gtk::Adjustment::new(
                        model.data.longitude, -180.0, 180.0, 0.01, 1.0, 0.0,
                    )),
                    set_digits: 4,
                    #[watch]
                    #[block_signal(lon_handler)]
                    set_value: model.data.longitude,
                    connect_changed[sender] => move |row| {
                        sender.input(SettingsInput::LonChanged(row.value()));
                    } @lon_handler,
                },

                adw::ActionRow {
                    set_title: &t!("settings.apply_location"),
                    set_subtitle: &t!("settings.apply_location.sub"),
                    add_suffix = &gtk::Button {
                        set_label: &t!("action.apply"),
                        set_valign: gtk::Align::Center,
                        add_css_class: "suggested-action",
                        connect_clicked[sender] => move |_| {
                            sender.input(SettingsInput::ApplyLocation);
                        },
                    },
                },
            },

            add = &adw::PreferencesGroup {
                set_title: &t!("settings.backend_group"),
                #[watch]
                set_sensitive: model.data.backend_ok,

                adw::ComboRow {
                    set_title: &t!("settings.backend"),
                    set_subtitle: &t!("settings.backend.sub"),
                    set_model: Some(&gtk::StringList::new(&[
                        "auto",
                        "hyprland",
                        "hyprsunset",
                        "wayland",
                    ])),
                    #[watch]
                    #[block_signal(backend_handler)]
                    set_selected: backend_index(model.data.backend),
                    connect_selected_notify[sender] => move |row| {
                        sender.input(SettingsInput::BackendChanged(row.selected()));
                    } @backend_handler,
                },

                adw::SpinRow {
                    set_title: &t!("settings.fade"),
                    set_adjustment: Some(&gtk::Adjustment::new(
                        model.data.startup_duration, 0.0, 60.0, 0.1, 1.0, 0.0,
                    )),
                    set_digits: 1,
                    #[watch]
                    #[block_signal(fade_handler)]
                    set_value: model.data.startup_duration,
                    connect_changed[sender] => move |row| {
                        sender.input(SettingsInput::StartupDurationChanged(row.value()));
                    } @fade_handler,
                },
            },

            add = &adw::PreferencesGroup {
                set_title: &t!("settings.app"),

                adw::ActionRow {
                    set_title: &t!("settings.autostart"),
                    set_subtitle: &t!("settings.autostart.sub"),
                    #[name(login_switch)]
                    add_suffix = &gtk::Switch {
                        set_valign: gtk::Align::Center,
                        #[watch]
                        #[block_signal(login_handler)]
                        set_active: model.data.start_on_login,
                        connect_state_set[sender] => move |_, active| {
                            sender.input(SettingsInput::StartOnLogin(active));
                            glib::Propagation::Proceed
                        } @login_handler,
                    },
                },

                adw::ActionRow {
                    set_title: &t!("settings.hide_on_close"),
                    set_subtitle: &t!("settings.hide_on_close.sub"),
                    add_suffix = &gtk::Switch {
                        set_valign: gtk::Align::Center,
                        #[watch]
                        #[block_signal(hide_handler)]
                        set_active: model.data.hide_on_close,
                        connect_state_set[sender] => move |_, active| {
                            sender.input(SettingsInput::HideOnClose(active));
                            glib::Propagation::Proceed
                        } @hide_handler,
                    },
                },
            },

            add = &adw::PreferencesGroup {
                set_title: &t!("settings.conflicts"),
                #[watch]
                set_visible: !model.data.conflicts.is_empty(),

                adw::ActionRow {
                    set_title: &t!("settings.conflicts.title"),
                    #[watch]
                    set_subtitle: &model.data.conflicts.join(" · "),
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SettingsModel {
            data: init,
            suppressing: false,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SettingsInput::Sync(init) => {
                self.suppressing = true;
                self.data = init;
                self.suppressing = false;
            }
            SettingsInput::ModeChanged(idx) if !self.suppressing => {
                let mode = index_to_mode(idx);
                self.data.transition_mode = mode;
                sender
                    .output(SettingsOutput::SetTransitionMode(mode))
                    .ok();
            }
            SettingsInput::BackendChanged(idx) if !self.suppressing => {
                let backend = index_to_backend(idx);
                self.data.backend = backend;
                sender.output(SettingsOutput::SetBackend(backend)).ok();
            }
            SettingsInput::SunsetChanged(s) if !self.suppressing => {
                self.data.sunset = s;
            }
            SettingsInput::SunriseChanged(s) if !self.suppressing => {
                self.data.sunrise = s;
            }
            SettingsInput::TransitionChanged(v) if !self.suppressing => {
                self.data.transition_duration = v;
            }
            SettingsInput::LatChanged(v) if !self.suppressing => {
                self.data.latitude = v;
            }
            SettingsInput::LonChanged(v) if !self.suppressing => {
                self.data.longitude = v;
            }
            SettingsInput::DayTempChanged(v) if !self.suppressing => {
                self.data.day_temp = v;
            }
            SettingsInput::NightTempChanged(v) if !self.suppressing => {
                self.data.night_temp = v;
            }
            SettingsInput::DayGammaChanged(v) if !self.suppressing => {
                self.data.day_gamma = v;
            }
            SettingsInput::NightGammaChanged(v) if !self.suppressing => {
                self.data.night_gamma = v;
            }
            SettingsInput::StartupDurationChanged(v) if !self.suppressing => {
                self.data.startup_duration = v;
                sender
                    .output(SettingsOutput::SetSmoothing { startup: v })
                    .ok();
            }
            SettingsInput::StartOnLogin(v) if !self.suppressing => {
                self.data.start_on_login = v;
                sender.output(SettingsOutput::SetStartOnLogin(v)).ok();
            }
            SettingsInput::HideOnClose(v) if !self.suppressing => {
                self.data.hide_on_close = v;
                sender.output(SettingsOutput::SetHideOnClose(v)).ok();
            }
            SettingsInput::ApplySchedule => {
                sender
                    .output(SettingsOutput::SetSchedule {
                        sunset: self.data.sunset.clone(),
                        sunrise: self.data.sunrise.clone(),
                        transition_minutes: self.data.transition_duration,
                    })
                    .ok();
            }
            SettingsInput::ApplyLocation => {
                sender
                    .output(SettingsOutput::SetLocation {
                        lat: self.data.latitude,
                        lon: self.data.longitude,
                    })
                    .ok();
            }
            SettingsInput::ApplyTemps => {
                sender
                    .output(SettingsOutput::SetDayNight {
                        day_temp: self.data.day_temp,
                        night_temp: self.data.night_temp,
                        day_gamma: self.data.day_gamma,
                        night_gamma: self.data.night_gamma,
                    })
                    .ok();
            }
            _ => {}
        }
    }
}

fn mode_index(mode: TransitionMode) -> u32 {
    match mode {
        TransitionMode::Geo => 0,
        TransitionMode::StartAt => 1,
        TransitionMode::FinishBy => 2,
        TransitionMode::Center => 3,
        TransitionMode::Static => 4,
    }
}

fn index_to_mode(idx: u32) -> TransitionMode {
    match idx {
        1 => TransitionMode::StartAt,
        2 => TransitionMode::FinishBy,
        3 => TransitionMode::Center,
        4 => TransitionMode::Static,
        _ => TransitionMode::Geo,
    }
}

fn backend_index(b: BackendKind) -> u32 {
    match b {
        BackendKind::Auto => 0,
        BackendKind::Hyprland => 1,
        BackendKind::Hyprsunset => 2,
        BackendKind::Wayland => 3,
    }
}

fn index_to_backend(idx: u32) -> BackendKind {
    match idx {
        1 => BackendKind::Hyprland,
        2 => BackendKind::Hyprsunset,
        3 => BackendKind::Wayland,
        _ => BackendKind::Auto,
    }
}

fn hhmm(t: &str) -> String {
    t.chars().take(5).collect()
}

/// Lista traduzida dos modos de transição (mesma ordem de `mode_index`).
fn mode_list() -> gtk::StringList {
    let list = gtk::StringList::new(&[]);
    for mode in [
        TransitionMode::Geo,
        TransitionMode::StartAt,
        TransitionMode::FinishBy,
        TransitionMode::Center,
        TransitionMode::Static,
    ] {
        list.append(&mode.label());
    }
    list
}
