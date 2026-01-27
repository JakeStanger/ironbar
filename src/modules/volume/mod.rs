mod config;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::volume::{self, Event};
use crate::config::{ModuleOrientation, ProfileUpdateEvent};
use crate::gtk_helpers::{IronbarLabelExt, OverflowLabel};
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{lock, module_impl, spawn};
use config::VolumeProfile;
use glib::subclass::prelude::*;
use glib::{Object, Properties};
use gtk::prelude::*;
use gtk::{
    Button, DropDown, Expression, Label, ListItem, Orientation, Scale, SignalListItemFactory,
    ToggleButton, gio,
};
use std::cell::RefCell;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::trace;

pub use config::VolumeModule;

#[derive(Debug, Clone)]
pub enum Update {
    SinkChange(String),
    SinkVolume(String, f64),
    SinkMute(String, bool),

    InputVolume(u32, f64),
    InputMute(u32, bool),
}

struct BarUiUpdate {
    muted: bool,
    description: String,
}

struct BtnMuteUiUpdate {
    muted: bool,
}

glib::wrapper! {
    pub struct DropdownItem(ObjectSubclass<DropdownItemData>);
}

impl DropdownItem {
    fn new(key: &str, value: &str) -> Self {
        Object::builder()
            .property("key", key)
            .property("value", value)
            .build()
    }
}

#[derive(Properties, Default)]
#[properties(wrapper_type = DropdownItem)]
pub struct DropdownItemData {
    #[property(get, set)]
    key: RefCell<String>,
    #[property(get, set)]
    value: RefCell<String>,
}

#[glib::derived_properties]
impl ObjectImpl for DropdownItemData {}

#[glib::object_subclass]
impl ObjectSubclass for DropdownItemData {
    const NAME: &'static str = "DropdownItem";
    type Type = DropdownItem;
}

impl Module<Button> for VolumeModule {
    type SendMessage = Event;
    type ReceiveMessage = Update;

    module_impl!("volume");

    fn on_create(&mut self) {
        self.profiles.setup_defaults(config::default_profiles());
    }

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

                trace!("initial syncs: {sinks:?}");

                let inputs = {
                    let inputs = client.sink_inputs();
                    let inputs = lock!(inputs);
                    inputs.iter().cloned().collect::<Vec<_>>()
                };

                trace!("initial inputs: {inputs:?}");

                for sink in sinks {
                    tx.send_update(Event::AddSink(sink)).await;
                }

                for input in inputs {
                    tx.send_update(Event::AddInput(input)).await;
                }

                // recv loop
                while let Ok(event) = rx.recv().await {
                    trace!("received event: {event:?}");
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
        let button_label = Label::builder()
            .use_markup(true)
            .justify(self.layout.justify.into())
            .build();

        let button = Button::new();
        button.set_child(Some(&button_label));

        {
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
            });
        }

        let rx = context.subscribe();

        let mut manager = {
            let format = self.format.clone();

            // attach to button as we want class there
            self.profiles.attach(
                &button,
                move |_, event: ProfileUpdateEvent<VolumeProfile, BarUiUpdate>| {
                    let icons = &event.profile.icons;
                    let label = format
                        .replace(
                            "{icon}",
                            if event.data.muted {
                                &icons.muted
                            } else {
                                &icons.volume
                            },
                        )
                        .replace("{percentage}", &event.value.to_string())
                        .replace("{name}", &event.data.description);

                    button_label.set_label_escaped(&label);
                },
            )
        };

        rx.recv_glib((), move |(), event| match event {
            Event::AddSink(sink) | Event::UpdateSink(sink) if sink.active => {
                manager.update(
                    sink.volume.percent() as i32,
                    BarUiUpdate {
                        muted: sink.muted,
                        description: sink.description,
                    },
                );
            }
            _ => {}
        });

        let popup = self
            .into_popup(context, info)
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        let container = gtk::Box::new(Orientation::Horizontal, 10);

        let sink_container = gtk::Box::new(Orientation::Vertical, 5);
        sink_container.add_css_class("device-box");

        let input_container = gtk::Box::new(Orientation::Vertical, 5);
        input_container.add_css_class("apps-box");

        container.append(&sink_container);
        container.append(&input_container);

        let options = gio::ListStore::new::<DropdownItem>();
        let factory = SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = Label::new(None);
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&label));
        });

        factory.connect_bind(move |_, list_item| {
            let dropdown_item = list_item
                .downcast_ref::<ListItem>()
                .expect("should be ListItem")
                .item()
                .and_downcast::<DropdownItem>()
                .expect("should be `DropdownItem`.");

            let label = list_item
                .downcast_ref::<ListItem>()
                .expect("should be ListItem")
                .child()
                .and_downcast::<Label>()
                .expect("should be a `Label`.");

            label.set_label(&dropdown_item.value().to_string());
        });

        let sink_selector = DropDown::new(Some(options.clone()), None::<Expression>);
        sink_selector.set_factory(Some(&factory));
        sink_selector.add_css_class("device-selector");

        {
            let tx = context.controller_tx.clone();

            sink_selector.connect_selected_notify(move |selector| {
                if let Some(item) = selector.selected_item().and_downcast_ref::<DropdownItem>() {
                    tx.send_spawn(Update::SinkChange(item.key()));
                }
            });
        }

        sink_container.append(&sink_selector);

        let slider = match self.sink_slider_orientation {
            ModuleOrientation::Horizontal => Scale::builder()
                .orientation(Orientation::Horizontal)
                .build(),
            ModuleOrientation::Vertical => Scale::builder()
                .orientation(Orientation::Vertical)
                .height_request(100)
                .inverted(true)
                .build(),
        };

        slider.add_css_class("slider");

        slider.set_range(0.0, self.max_volume);
        slider.set_value(50.0);
        sink_container.append(&slider);

        {
            let tx = context.controller_tx.clone();
            let selector = sink_selector.clone();

            let scale = slider.clone();
            scale.connect_value_changed(move |scale| {
                if scale.has_css_class("dragging")
                    && let Some(sink) = selector.selected_item().and_downcast_ref::<DropdownItem>()
                {
                    // GTK will send values outside min/max range
                    let val = scale.value().clamp(0.0, self.max_volume);
                    tx.send_spawn(Update::SinkVolume(sink.key(), val));
                }
            });
        }

        let btn_mute = ToggleButton::new();
        btn_mute.add_css_class("btn-mute");
        sink_container.append(&btn_mute);

        let mut manager = self.profiles.attach(
            &btn_mute,
            move |btn_mute, event: ProfileUpdateEvent<VolumeProfile, BtnMuteUiUpdate>| {
                btn_mute.set_active(event.data.muted);

                let icons = &event.profile.icons;
                btn_mute.set_label(if event.data.muted {
                    &icons.muted
                } else {
                    &icons.volume
                });
            },
        );

        {
            let tx = context.controller_tx.clone();
            let selector = sink_selector.clone();

            btn_mute.connect_toggled(move |btn| {
                if let Some(sink) = selector.selected_item().and_downcast_ref::<DropdownItem>() {
                    let muted = btn.is_active();
                    tx.send_spawn(Update::SinkMute(sink.key(), muted));
                }
            });
        }

        let mut inputs = HashMap::new();
        let mut sinks = vec![];

        context
            .subscribe()
            .recv_glib(&input_container, move |input_container, event| {
                match event {
                    Event::AddSink(info) => {
                        options.append(&DropdownItem::new(&info.name, &info.description));

                        if info.active {
                            sink_selector.set_selected(sinks.len() as u32);
                            slider.set_value(info.volume.percent());

                            manager.update(
                                info.volume.percent() as i32,
                                BtnMuteUiUpdate { muted: info.muted },
                            );
                        }

                        sinks.push(info);
                    }
                    Event::UpdateSink(info) => {
                        if info.active
                            && let Some(pos) = sinks.iter().position(|s| s.name == info.name)
                        {
                            sink_selector.set_selected(pos as u32);

                            if !slider.has_css_class("dragging") {
                                slider.set_value(info.volume.percent());
                            }

                            manager.update(
                                info.volume.percent() as i32,
                                BtnMuteUiUpdate { muted: info.muted },
                            );
                        }
                    }
                    Event::RemoveSink(name) => {
                        if let Some(pos) = sinks.iter().position(|s| s.name == name) {
                            options.remove(pos as u32);
                            sinks.remove(pos);
                        }
                    }

                    Event::AddInput(info) => {
                        let index = info.index;

                        let item_container = gtk::Box::new(Orientation::Vertical, 0);
                        item_container.add_css_class("app-box");

                        let title_label = OverflowLabel::new(
                            Label::new(None),
                            self.truncate,
                            self.marquee.clone(),
                        );
                        title_label.label().add_css_class("title");
                        title_label.set_label_escaped(&info.name);
                        item_container.append(title_label.widget());

                        let slider = Scale::builder().sensitive(info.can_set_volume).build();
                        slider.set_range(0.0, self.max_volume);
                        slider.set_value(info.volume.percent());
                        slider.add_css_class("slider");

                        {
                            let tx = context.controller_tx.clone();
                            slider.connect_value_changed(move |scale| {
                                if scale.has_css_class("dragging") {
                                    // GTK will send values outside min/max range
                                    let val = scale.value().clamp(0.0, self.max_volume);
                                    tx.send_spawn(Update::InputVolume(index, val));
                                }
                            });
                        }

                        let btn_mute = ToggleButton::new();
                        btn_mute.add_css_class("btn-mute");

                        manager.update(
                            info.volume.percent() as i32,
                            BtnMuteUiUpdate { muted: info.muted },
                        );

                        {
                            let tx = context.controller_tx.clone();
                            btn_mute.connect_toggled(move |btn| {
                                let muted = btn.is_active();
                                tx.send_spawn(Update::InputMute(index, muted));
                            });
                        }

                        item_container.append(&slider);
                        item_container.append(&btn_mute);

                        input_container.append(&item_container);

                        inputs.insert(
                            info.index,
                            InputUi {
                                container: item_container,
                                title_label,
                                slider,
                                label_raw: info.name.clone(),
                            },
                        );
                    }
                    Event::UpdateInput(info) => {
                        if let Some(ui) = inputs.get_mut(&info.index) {
                            if ui.label_raw != info.name {
                                ui.title_label.set_label_escaped(&info.name);
                                ui.label_raw = info.name.clone();
                            }

                            if !ui.slider.has_css_class("dragging") {
                                ui.slider.set_value(info.volume.percent());
                            }

                            ui.slider.set_sensitive(info.can_set_volume);
                            manager.update(
                                info.volume.percent() as i32,
                                BtnMuteUiUpdate { muted: info.muted },
                            );
                        }
                    }
                    Event::RemoveInput(index) => {
                        if let Some(ui) = inputs.remove(&index) {
                            input_container.remove(&ui.container);
                        }
                    }
                }
            });

        Some(container)
    }
}

struct InputUi {
    container: gtk::Box,
    title_label: OverflowLabel,
    slider: Scale,
    // Store original (unformatted) title to detect change when marquee is enabled
    label_raw: String,
}
