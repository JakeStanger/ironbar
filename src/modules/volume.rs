use crate::clients::volume::{self, Event};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::ImageProvider;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{glib_recv, lock, module_impl, send_async, spawn, try_send};
use glib::Propagation;
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Button, CellRendererText, ComboBoxText, Image, Label, Orientation, Scale,
    ToggleButton,
};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Deserialize)]
pub struct VolumeModule {
    /// Maximum value to allow volume sliders to reach.
    /// Pulse supports values > 100 but this may result in distortion.
    ///
    /// **Default**: `100`
    #[serde(default = "default_max_volume")]
    max_volume: f64,

    #[serde(default = "default_icon_size")]
    icon_size: i32,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

const fn default_max_volume() -> f64 {
    100.0
}

const fn default_icon_size() -> i32 {
    24
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
                    send_async!(tx, ModuleUpdateEvent::Update(Event::AddSink(sink)));
                }

                for input in inputs {
                    send_async!(
                        tx,
                        ModuleUpdateEvent::Update(Event::AddInput(input.clone()))
                    );
                }

                // recv loop
                while let Ok(event) = rx.recv().await {
                    send_async!(tx, ModuleUpdateEvent::Update(event));
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
                try_send!(tx, ModuleUpdateEvent::TogglePopup(button.popup_id()));
            });
        }

        {
            let rx = context.subscribe();
            let icon_theme = info.icon_theme.clone();

            let image_icon = Image::new();
            image_icon.add_class("icon");
            button.set_image(Some(&image_icon));

            glib_recv!(rx, event => {
                match event {
                    Event::AddSink(sink) | Event::UpdateSink(sink) if sink.active => {
                        ImageProvider::parse(
                            &determine_volume_icon(sink.muted, sink.volume),
                            &icon_theme,
                            false,
                            self.icon_size,
                        ).map(|provider| provider.load_into_image(image_icon.clone()));
                    },
                    _ => {},
                }
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
        info: &ModuleInfo,
    ) -> Option<GtkBox>
    where
        Self: Sized,
    {
        let container = GtkBox::new(Orientation::Horizontal, 10);

        let sink_container = GtkBox::new(Orientation::Vertical, 5);
        sink_container.add_class("device-box");

        let input_container = GtkBox::new(Orientation::Vertical, 5);
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
                    try_send!(tx, Update::SinkChange(name.into()));
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
                    try_send!(tx, Update::SinkVolume(sink.into(), val));
                }

                Propagation::Proceed
            });
        }

        let btn_mute = ToggleButton::new();
        btn_mute.add_class("btn-mute");
        let btn_mute_icon = Image::new();
        btn_mute.set_image(Some(&btn_mute_icon));
        sink_container.add(&btn_mute);

        {
            let tx = tx.clone();
            let selector = sink_selector.clone();

            btn_mute.connect_toggled(move |btn| {
                if let Some(sink) = selector.active_id() {
                    let muted = btn.is_active();
                    try_send!(tx, Update::SinkMute(sink.into(), muted));
                }
            });
        }

        container.show_all();

        let mut inputs = HashMap::new();

        {
            let icon_theme = info.icon_theme.clone();
            let input_container = input_container.clone();

            let mut sinks = vec![];

            glib_recv!(rx, event => {
                match event {
                    Event::AddSink(info) => {
                        sink_selector.append(Some(&info.name), &info.description);

                        if info.active {
                            sink_selector.set_active(Some(sinks.len() as u32));
                            slider.set_value(info.volume);

                            btn_mute.set_active(info.muted);
                            ImageProvider::parse(
                                &determine_volume_icon(info.muted, info.volume),
                                &icon_theme,
                                false,
                                self.icon_size,
                            ).map(|provider| provider.load_into_image(btn_mute_icon.clone()));
                        }

                        sinks.push(info);
                    }
                    Event::UpdateSink(info) => {
                        if info.active {
                            if let Some(pos) = sinks.iter().position(|s| s.name == info.name) {
                                sink_selector.set_active(Some(pos as u32));
                                slider.set_value(info.volume);

                                btn_mute.set_active(info.muted);
                                ImageProvider::parse(
                                    &determine_volume_icon(info.muted, info.volume),
                                    &icon_theme,
                                    false,
                                    self.icon_size,
                                ).map(|provider| provider.load_into_image(btn_mute_icon.clone()));
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

                        let item_container = GtkBox::new(Orientation::Vertical, 0);
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
                                try_send!(tx, Update::InputVolume(index, val));

                                Propagation::Proceed
                            });
                        }

                        let btn_mute = ToggleButton::new();
                        btn_mute.add_class("btn-mute");
                        let btn_mute_icon = Image::new();
                        btn_mute.set_image(Some(&btn_mute_icon));

                        btn_mute.set_active(info.muted);
                        ImageProvider::parse(
                            &determine_volume_icon(info.muted, info.volume),
                            &icon_theme,
                            false,
                            self.icon_size,
                        ).map(|provider| provider.load_into_image(btn_mute_icon.clone()));

                        {
                            let tx = tx.clone();
                            btn_mute.connect_toggled(move |btn| {
                                let muted = btn.is_active();
                                try_send!(tx, Update::InputMute(index, muted));
                            });
                        }

                        item_container.add(&label);
                        item_container.add(&slider);
                        item_container.add(&btn_mute);
                        item_container.show_all();

                        input_container.add(&item_container);

                        inputs.insert(info.index, InputUi {
                            container: item_container,
                            label,
                            slider,
                            btn_mute_icon,
                        });
                    }
                    Event::UpdateInput(info) => {
                        if let Some(ui) = inputs.get(&info.index) {
                            ui.label.set_label(&info.name);
                            ui.slider.set_value(info.volume);
                            ui.slider.set_sensitive(info.can_set_volume);
                            ImageProvider::parse(
                                &determine_volume_icon(info.muted, info.volume),
                                &icon_theme,
                                false,
                                self.icon_size,
                            ).map(|provider| provider.load_into_image(ui.btn_mute_icon.clone()));
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
    container: GtkBox,
    label: Label,
    slider: Scale,
    btn_mute_icon: Image,
}

fn determine_volume_icon(muted: bool, volume: f64) -> String {
    let icon_variant = if muted {
        "muted"
    } else if volume <= 33.3333 {
        "low"
    } else if volume <= 66.6667 {
        "medium"
    } else {
        "high"
    };
    format!("audio-volume-{icon_variant}-symbolic")
}
