use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::volume::{self, Event};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{lock, module_impl, spawn};
use glib::Propagation;
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{Button, CellRendererText, ComboBoxText, Label, Orientation, Scale, ToggleButton};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VolumeModule {
    /// The format string to use for the widget button label.
    /// For available tokens, see [below](#formatting-tokens).
    ///
    /// **Default**: `{icon} {percentage}%`
    #[serde(default = "default_format")]
    format: String,

    /// Maximum value to allow volume sliders to reach.
    /// Pulse supports values > 100 but this may result in distortion.
    ///
    /// **Default**: `100`
    #[serde(default = "default_max_volume")]
    max_volume: f64,

    /// Volume state icons.
    ///
    /// See [icons](#icons).
    #[serde(default)]
    icons: Icons,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

fn default_format() -> String {
    String::from("{icon} {percentage}%")
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Icons {
    /// Icon to show for high volume levels.
    ///
    /// **Default**: `󰕾`
    #[serde(default = "default_icon_volume_high")]
    volume_high: String,

    /// Icon to show for medium volume levels.
    ///
    /// **Default**: `󰖀`
    #[serde(default = "default_icon_volume_medium")]
    volume_medium: String,

    /// Icon to show for low volume levels.
    ///
    /// **Default**: `󰕿`
    #[serde(default = "default_icon_volume_low")]
    volume_low: String,

    /// Icon to show for muted outputs.
    ///
    /// **Default**: `󰝟`
    #[serde(default = "default_icon_muted")]
    muted: String,
}

impl Icons {
    fn volume_icon(&self, volume_percent: f64) -> &str {
        match volume_percent as u32 {
            0..=33 => &self.volume_low,
            34..=66 => &self.volume_medium,
            67.. => &self.volume_high,
        }
    }
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            volume_high: default_icon_volume_high(),
            volume_medium: default_icon_volume_medium(),
            volume_low: default_icon_volume_low(),
            muted: default_icon_muted(),
        }
    }
}

const fn default_max_volume() -> f64 {
    100.0
}

fn default_icon_volume_high() -> String {
    String::from("󰕾")
}

fn default_icon_volume_medium() -> String {
    String::from("󰖀")
}

fn default_icon_volume_low() -> String {
    String::from("󰕿")
}

fn default_icon_muted() -> String {
    String::from("󰝟")
}

#[derive(Debug, Clone)]
pub enum Update {
    SinkChange(String),
    SinkVolume(String, f64),
    SinkMute(String, bool),

    InputVolume(u32, f64),
    InputMute(u32, bool),
}

impl Module<Button> for VolumeModule {
    type SendMessage = Event;
    type ReceiveMessage = Update;

    module_impl!("volume");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> color_eyre::Result<()>
    where
        <Self as Module<Button>>::SendMessage: Clone,
    {
        let client = context.client::<volume::Client>();

        {
            let client = client.clone();
            let mut rx = client.subscribe();
            let tx = context.tx.clone();

            spawn(async move {
                // init
                let sinks = {
                    let sinks = client.sinks();
                    let sinks = lock!(sinks);
                    sinks.iter().cloned().collect::<Vec<_>>()
                };

                let inputs = {
                    let inputs = client.sink_inputs();
                    let inputs = lock!(inputs);
                    inputs.iter().cloned().collect::<Vec<_>>()
                };

                for sink in sinks {
                    tx.send_update(Event::AddSink(sink)).await;
                }

                for input in inputs {
                    tx.send_update(Event::AddInput(input.clone())).await;
                }

                // recv loop
                while let Ok(event) = rx.recv().await {
                    tx.send_update(event).await;
                }
            });
        }

        // ui events
        spawn(async move {
            while let Some(update) = rx.recv().await {
                match update {
                    Update::SinkChange(name) => client.set_default_sink(&name),
                    Update::SinkVolume(name, volume) => client.set_sink_volume(&name, volume),
                    Update::SinkMute(name, muted) => client.set_sink_muted(&name, muted),
                    Update::InputVolume(index, volume) => client.set_input_volume(index, volume),
                    Update::InputMute(index, muted) => client.set_input_muted(index, muted),
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> color_eyre::Result<ModuleParts<Button>>
    where
        <Self as Module<Button>>::SendMessage: Clone,
    {
        let button = Button::new();

        {
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
            });
        }

        {
            let rx = context.subscribe();
            let icons = self.icons.clone();
            let button = button.clone();

            let format = self.format.clone();

            rx.recv_glib(move |event| match event {
                Event::AddSink(sink) | Event::UpdateSink(sink) if sink.active => {
                    let label = format
                        .replace(
                            "{icon}",
                            if sink.muted {
                                &icons.muted
                            } else {
                                icons.volume_icon(sink.volume)
                            },
                        )
                        .replace("{percentage}", &sink.volume.to_string())
                        .replace("{name}", &sink.description);

                    button.set_label(&label);
                }
                _ => {}
            });
        }

        let popup = self
            .into_popup(
                context.controller_tx.clone(),
                context.subscribe(),
                context,
                info,
            )
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        tx: mpsc::Sender<Self::ReceiveMessage>,
        rx: tokio::sync::broadcast::Receiver<Self::SendMessage>,
        _context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        let container = gtk::Box::new(Orientation::Horizontal, 10);

        let sink_container = gtk::Box::new(Orientation::Vertical, 5);
        sink_container.add_class("device-box");

        let input_container = gtk::Box::new(Orientation::Vertical, 5);
        input_container.add_class("apps-box");

        container.add(&sink_container);
        container.add(&input_container);

        let sink_selector = ComboBoxText::new();
        sink_selector.add_class("device-selector");

        let renderer = sink_selector
            .cells()
            .first()
            .expect("to exist")
            .clone()
            .downcast::<CellRendererText>()
            .expect("to be valid cast");

        renderer.set_width_chars(20);
        renderer.set_ellipsize(EllipsizeMode::End);

        {
            let tx = tx.clone();
            sink_selector.connect_changed(move |selector| {
                if let Some(name) = selector.active_id() {
                    tx.send_spawn(Update::SinkChange(name.into()));
                }
            });
        }

        sink_container.add(&sink_selector);

        let slider = Scale::builder()
            .orientation(Orientation::Vertical)
            .height_request(100)
            .inverted(true)
            .build();

        slider.add_class("slider");

        slider.set_range(0.0, self.max_volume);
        slider.set_value(50.0);
        sink_container.add(&slider);

        {
            let tx = tx.clone();
            let selector = sink_selector.clone();

            slider.connect_button_release_event(move |scale, _| {
                if let Some(sink) = selector.active_id() {
                    // GTK will send values outside min/max range
                    let val = scale.value().clamp(0.0, self.max_volume);
                    tx.send_spawn(Update::SinkVolume(sink.into(), val));
                }

                Propagation::Proceed
            });
        }

        let btn_mute = ToggleButton::new();
        btn_mute.add_class("btn-mute");
        sink_container.add(&btn_mute);

        {
            let tx = tx.clone();
            let selector = sink_selector.clone();

            btn_mute.connect_toggled(move |btn| {
                if let Some(sink) = selector.active_id() {
                    let muted = btn.is_active();
                    tx.send_spawn(Update::SinkMute(sink.into(), muted));
                }
            });
        }

        container.show_all();

        let mut inputs = HashMap::new();

        {
            let input_container = input_container.clone();

            let mut sinks = vec![];

            rx.recv_glib(move |event| {
                match event {
                    Event::AddSink(info) => {
                        sink_selector.append(Some(&info.name), &info.description);

                        if info.active {
                            sink_selector.set_active(Some(sinks.len() as u32));
                            slider.set_value(info.volume);

                            btn_mute.set_active(info.muted);
                            btn_mute.set_label(if info.muted {
                                &self.icons.muted
                            } else {
                                self.icons.volume_icon(info.volume)
                            });
                        }

                        sinks.push(info);
                    }
                    Event::UpdateSink(info) => {
                        if info.active {
                            if let Some(pos) = sinks.iter().position(|s| s.name == info.name) {
                                sink_selector.set_active(Some(pos as u32));
                                slider.set_value(info.volume);

                                btn_mute.set_active(info.muted);
                                btn_mute.set_label(if info.muted {
                                    &self.icons.muted
                                } else {
                                    self.icons.volume_icon(info.volume)
                                });
                            }
                        }
                    }
                    Event::RemoveSink(name) => {
                        if let Some(pos) = sinks.iter().position(|s| s.name == name) {
                            ComboBoxTextExt::remove(&sink_selector, pos as i32);
                            sinks.remove(pos);
                        }
                    }

                    Event::AddInput(info) => {
                        let index = info.index;

                        let item_container = gtk::Box::new(Orientation::Vertical, 0);
                        item_container.add_class("app-box");

                        let label = Label::new(Some(&info.name));
                        label.add_class("title");

                        let slider = Scale::builder().sensitive(info.can_set_volume).build();
                        slider.set_range(0.0, self.max_volume);
                        slider.set_value(info.volume);
                        slider.add_class("slider");

                        {
                            let tx = tx.clone();
                            slider.connect_button_release_event(move |scale, _| {
                                // GTK will send values outside min/max range
                                let val = scale.value().clamp(0.0, self.max_volume);
                                tx.send_spawn(Update::InputVolume(index, val));

                                Propagation::Proceed
                            });
                        }

                        let btn_mute = ToggleButton::new();
                        btn_mute.add_class("btn-mute");

                        btn_mute.set_active(info.muted);
                        btn_mute.set_label(if info.muted {
                            &self.icons.muted
                        } else {
                            self.icons.volume_icon(info.volume)
                        });

                        {
                            let tx = tx.clone();
                            btn_mute.connect_toggled(move |btn| {
                                let muted = btn.is_active();
                                tx.send_spawn(Update::InputMute(index, muted));
                            });
                        }

                        item_container.add(&label);
                        item_container.add(&slider);
                        item_container.add(&btn_mute);
                        item_container.show_all();

                        input_container.add(&item_container);

                        inputs.insert(
                            info.index,
                            InputUi {
                                container: item_container,
                                label,
                                slider,
                                btn_mute,
                            },
                        );
                    }
                    Event::UpdateInput(info) => {
                        if let Some(ui) = inputs.get(&info.index) {
                            ui.label.set_label(&info.name);
                            ui.slider.set_value(info.volume);
                            ui.slider.set_sensitive(info.can_set_volume);
                            ui.btn_mute.set_label(if info.muted {
                                &self.icons.muted
                            } else {
                                self.icons.volume_icon(info.volume)
                            });
                        }
                    }
                    Event::RemoveInput(index) => {
                        if let Some(ui) = inputs.remove(&index) {
                            input_container.remove(&ui.container);
                        }
                    }
                }
            });
        }

        Some(container)
    }
}

struct InputUi {
    container: gtk::Box,
    label: Label,
    slider: Scale,
    btn_mute: ToggleButton,
}
