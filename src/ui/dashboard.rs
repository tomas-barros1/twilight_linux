//! Página principal: status, toggle, sliders, preview e presets.

use std::cell::Cell;
use std::rc::Rc;

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk, gtk::glib};

use crate::backend::{Period, UiPreset};
use crate::color::{intensity_to_kelvin, kelvin_to_intensity, kelvin_to_rgb};

#[derive(Debug)]
pub struct DashboardInit {
    pub enabled: bool,
    pub temperature: u32,
    pub intensity: u32,
    pub gamma: f64,
    pub period: Period,
    pub preset: UiPreset,
    pub preset_label: String,
    pub next_period: String,
    pub backend_ok: bool,
    pub error: Option<String>,
    pub conflicts: Vec<String>,
}

#[derive(Debug)]
pub enum DashboardInput {
    Sync(DashboardInit),
    ToggleEnabled(bool),
    TempChanged(u32),
    IntensityChanged(u32),
    GammaChanged(f64),
    SelectPreset(UiPreset),
    Refresh,
}

#[derive(Debug)]
pub enum DashboardOutput {
    SetEnabled(bool),
    SetTemperature(u32),
    SetIntensity(u32),
    SetGamma(f64),
    ApplyPreset(UiPreset),
    RequestRefresh,
}

pub struct DashboardModel {
    pub enabled: bool,
    pub temperature: u32,
    pub intensity: u32,
    pub gamma: f64,
    pub period: Period,
    pub preset: UiPreset,
    pub preset_label: String,
    pub next_period: String,
    pub backend_ok: bool,
    pub error: Option<String>,
    pub conflicts: Vec<String>,
    suppressing: bool,
    preview_temp: Rc<Cell<u32>>,
    preview: gtk::DrawingArea,
}

#[relm4::component(pub)]
impl SimpleComponent for DashboardModel {
    type Init = DashboardInit;
    type Input = DashboardInput;
    type Output = DashboardOutput;

    view! {
        adw::PreferencesPage {
            set_title: "Filtro",

            add = &adw::PreferencesGroup {
                set_title: "Status",
                set_description: Some("Filtro aplicado pelo sunsetr com config isolada do Twilight."),

                adw::ActionRow {
                    set_title: "Estado atual",
                    #[watch]
                    set_subtitle: &status_subtitle(&model),

                    add_suffix = &gtk::Button {
                        set_icon_name: "view-refresh-symbolic",
                        set_valign: gtk::Align::Center,
                        set_tooltip_text: Some("Atualizar"),
                        add_css_class: "flat",
                        connect_clicked[sender] => move |_| {
                            sender.input(DashboardInput::Refresh);
                        },
                    },
                },

                adw::ActionRow {
                    set_title: "Filtro de luz azul",
                    set_subtitle: "Liga ou desliga o filtro no compositor",

                    add_suffix = &gtk::Switch {
                        set_valign: gtk::Align::Center,
                        #[watch]
                        #[block_signal(enable_handler)]
                        set_active: model.enabled,
                        connect_state_set[sender] => move |_, active| {
                            sender.input(DashboardInput::ToggleEnabled(active));
                            glib::Propagation::Proceed
                        } @enable_handler,
                    },
                },
            },

            add = &adw::PreferencesGroup {
                set_title: "Aparência",
                #[watch]
                set_sensitive: model.enabled && model.backend_ok,

                #[name(preview)]
                gtk::DrawingArea {
                    set_content_width: 320,
                    set_content_height: 56,
                    set_hexpand: true,
                    set_margin_top: 8,
                    set_margin_bottom: 4,
                    set_margin_start: 12,
                    set_margin_end: 12,
                    add_css_class: "card",
                },

                gtk::Label {
                    #[watch]
                    set_label: &format!("Pré-visualização · {} K", model.temperature),
                    set_margin_bottom: 8,
                    set_css_classes: &["dim-label", "caption"],
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,
                    set_margin_start: 12,
                    set_margin_end: 12,
                    set_margin_bottom: 8,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        gtk::Label {
                            set_label: "Temperatura",
                            set_halign: gtk::Align::Start,
                            set_hexpand: true,
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &format!("{} K", model.temperature),
                            add_css_class: "dim-label",
                        },
                    },
                    #[name(temp_scale)]
                    gtk::Scale {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_hexpand: true,
                        set_draw_value: false,
                        set_range: (1000.0, 6500.0),
                        set_increments: (50.0, 200.0),
                        #[watch]
                        #[block_signal(temp_handler)]
                        set_value: model.temperature as f64,
                        connect_value_changed[sender] => move |s| {
                            sender.input(DashboardInput::TempChanged(s.value().round() as u32));
                        } @temp_handler,
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,
                    set_margin_start: 12,
                    set_margin_end: 12,
                    set_margin_bottom: 8,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        gtk::Label {
                            set_label: "Intensidade",
                            set_halign: gtk::Align::Start,
                            set_hexpand: true,
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &format!("{}%", model.intensity),
                            add_css_class: "dim-label",
                        },
                    },
                    #[name(intensity_scale)]
                    gtk::Scale {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_hexpand: true,
                        set_draw_value: false,
                        set_range: (0.0, 100.0),
                        set_increments: (1.0, 5.0),
                        #[watch]
                        #[block_signal(intensity_handler)]
                        set_value: model.intensity as f64,
                        connect_value_changed[sender] => move |s| {
                            sender.input(DashboardInput::IntensityChanged(s.value().round() as u32));
                        } @intensity_handler,
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,
                    set_margin_start: 12,
                    set_margin_end: 12,
                    set_margin_bottom: 12,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        gtk::Label {
                            set_label: "Brilho (gamma)",
                            set_halign: gtk::Align::Start,
                            set_hexpand: true,
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &format!("{:.0}%", model.gamma),
                            add_css_class: "dim-label",
                        },
                    },
                    #[name(gamma_scale)]
                    gtk::Scale {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_hexpand: true,
                        set_draw_value: false,
                        set_range: (10.0, 150.0),
                        set_increments: (1.0, 5.0),
                        #[watch]
                        #[block_signal(gamma_handler)]
                        set_value: model.gamma,
                        connect_value_changed[sender] => move |s| {
                            sender.input(DashboardInput::GammaChanged(s.value()));
                        } @gamma_handler,
                    },
                },
            },

            add = &adw::PreferencesGroup {
                set_title: "Presets",
                set_description: Some("Dia, pôr do sol e noite usam modo estático. Agendado restaura o horário."),
                #[watch]
                set_sensitive: model.enabled && model.backend_ok,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,
                    set_homogeneous: true,
                    set_margin_top: 8,
                    set_margin_bottom: 12,
                    set_margin_start: 12,
                    set_margin_end: 12,
                    set_hexpand: true,

                    #[name(btn_day)]
                    gtk::ToggleButton {
                        set_label: "Dia",
                        set_hexpand: true,
                        #[watch]
                        #[block_signal(day_handler)]
                        set_active: model.preset == UiPreset::Day,
                        connect_toggled[sender] => move |btn| {
                            if btn.is_active() {
                                sender.input(DashboardInput::SelectPreset(UiPreset::Day));
                            }
                        } @day_handler,
                    },
                    #[name(btn_sunset)]
                    gtk::ToggleButton {
                        set_label: "Pôr do sol",
                        set_hexpand: true,
                        #[watch]
                        #[block_signal(sunset_handler)]
                        set_active: model.preset == UiPreset::Sunset,
                        connect_toggled[sender] => move |btn| {
                            if btn.is_active() {
                                sender.input(DashboardInput::SelectPreset(UiPreset::Sunset));
                            }
                        } @sunset_handler,
                    },
                    #[name(btn_night)]
                    gtk::ToggleButton {
                        set_label: "Noite",
                        set_hexpand: true,
                        #[watch]
                        #[block_signal(night_handler)]
                        set_active: model.preset == UiPreset::Night,
                        connect_toggled[sender] => move |btn| {
                            if btn.is_active() {
                                sender.input(DashboardInput::SelectPreset(UiPreset::Night));
                            }
                        } @night_handler,
                    },
                    #[name(btn_schedule)]
                    gtk::ToggleButton {
                        set_label: "Agendado",
                        set_hexpand: true,
                        #[watch]
                        #[block_signal(schedule_handler)]
                        set_active: model.preset == UiPreset::Schedule,
                        connect_toggled[sender] => move |btn| {
                            if btn.is_active() {
                                sender.input(DashboardInput::SelectPreset(UiPreset::Schedule));
                            }
                        } @schedule_handler,
                    },
                },
            },

            add = &adw::PreferencesGroup {
                set_title: "Avisos",
                #[watch]
                set_visible: model.error.is_some() || !model.conflicts.is_empty() || !model.backend_ok,

                adw::ActionRow {
                    #[watch]
                    set_title: if !model.backend_ok {
                        "sunsetr não encontrado"
                    } else if model.error.is_some() {
                        "Erro do backend"
                    } else {
                        "Possível conflito"
                    },
                    #[watch]
                    set_subtitle: &warning_text(&model),
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let _ = (&root, &sender);
        let preview_temp = Rc::new(Cell::new(init.temperature));
        let model = DashboardModel {
            enabled: init.enabled,
            temperature: init.temperature,
            intensity: init.intensity,
            gamma: init.gamma,
            period: init.period,
            preset: init.preset,
            preset_label: init.preset_label,
            next_period: init.next_period,
            backend_ok: init.backend_ok,
            error: init.error,
            conflicts: init.conflicts,
            suppressing: false,
            preview_temp: preview_temp.clone(),
            preview: gtk::DrawingArea::new(), // replaced after view_output
        };

        let widgets = view_output!();

        let draw_temp = preview_temp.clone();
        widgets.preview.set_draw_func(move |_, cr, width, height| {
            let kelvin = draw_temp.get();
            let (r, g, b) = kelvin_to_rgb(kelvin);
            cr.set_source_rgb(r, g, b);
            cr.rectangle(0.0, 0.0, width as f64, height as f64);
            let _ = cr.fill();

            cr.set_source_rgba(0.0, 0.0, 0.0, 0.25);
            cr.set_line_width(1.0);
            cr.rectangle(0.5, 0.5, width as f64 - 1.0, height as f64 - 1.0);
            let _ = cr.stroke();
        });

        widgets.btn_sunset.set_group(Some(&widgets.btn_day));
        widgets.btn_night.set_group(Some(&widgets.btn_day));
        widgets.btn_schedule.set_group(Some(&widgets.btn_day));

        let mut model = model;
        model.preview = widgets.preview.clone();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DashboardInput::Sync(init) => {
                self.suppressing = true;
                self.enabled = init.enabled;
                self.temperature = init.temperature;
                self.intensity = init.intensity;
                self.gamma = init.gamma;
                self.period = init.period;
                self.preset = init.preset;
                self.preset_label = init.preset_label;
                self.next_period = init.next_period;
                self.backend_ok = init.backend_ok;
                self.error = init.error;
                self.conflicts = init.conflicts;
                self.preview_temp.set(self.temperature);
                self.preview.queue_draw();
                self.suppressing = false;
            }
            DashboardInput::ToggleEnabled(active) => {
                if self.suppressing {
                    return;
                }
                self.enabled = active;
                sender.output(DashboardOutput::SetEnabled(active)).ok();
            }
            DashboardInput::TempChanged(temp) => {
                if self.suppressing {
                    return;
                }
                self.temperature = temp.clamp(1000, 6500);
                self.intensity = kelvin_to_intensity(self.temperature);
                self.preview_temp.set(self.temperature);
                self.preview.queue_draw();
                sender
                    .output(DashboardOutput::SetTemperature(self.temperature))
                    .ok();
            }
            DashboardInput::IntensityChanged(intensity) => {
                if self.suppressing {
                    return;
                }
                self.intensity = intensity.min(100);
                self.temperature = intensity_to_kelvin(self.intensity);
                self.preview_temp.set(self.temperature);
                self.preview.queue_draw();
                sender
                    .output(DashboardOutput::SetIntensity(self.intensity))
                    .ok();
            }
            DashboardInput::GammaChanged(gamma) => {
                if self.suppressing {
                    return;
                }
                self.gamma = gamma.clamp(10.0, 150.0);
                sender.output(DashboardOutput::SetGamma(self.gamma)).ok();
            }
            DashboardInput::SelectPreset(preset) => {
                if self.suppressing {
                    return;
                }
                self.preset = preset;
                self.preset_label = preset.label().to_string();
                sender.output(DashboardOutput::ApplyPreset(preset)).ok();
            }
            DashboardInput::Refresh => {
                sender.output(DashboardOutput::RequestRefresh).ok();
            }
        }
    }

}

fn status_subtitle(model: &DashboardModel) -> String {
    if !model.backend_ok {
        return "Instale o pacote sunsetr para aplicar o filtro no desktop.".into();
    }
    if !model.enabled {
        return "Filtro desligado".into();
    }
    let next = if model.next_period.is_empty() {
        String::new()
    } else {
        format!(" · próximo: {}", short_time(&model.next_period))
    };
    format!(
        "{} · {} K · gamma {:.0}% · {}{}",
        model.period.label(),
        model.temperature,
        model.gamma,
        model.preset_label,
        next
    )
}

fn warning_text(model: &DashboardModel) -> String {
    if !model.backend_ok {
        return "sudo pacman -S sunsetr".into();
    }
    if let Some(err) = &model.error {
        return err.clone();
    }
    model.conflicts.join(" · ")
}

fn short_time(iso: &str) -> String {
    if let Some(t) = iso.split('T').nth(1) {
        t.chars().take(5).collect()
    } else {
        iso.to_string()
    }
}
