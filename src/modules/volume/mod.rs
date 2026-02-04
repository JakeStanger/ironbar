mod config;

use std::cell::RefCell;
use std::collections::HashMap;

use glib::subclass::prelude::*;
use glib::{Object, Properties};
use gtk::prelude::*;
use gtk::{
    Button, DropDown, Expression, Label, ListItem, Orientation, Scale, SignalListItemFactory,
    ToggleButton, gio,
};
use tokio::sync::mpsc::{self, Sender};
use tracing::trace;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::volume::{self, Event};
use crate::config::{ModuleOrientation, ProfileUpdateEvent};
use crate::gtk_helpers::{IronbarLabelExt, OverflowLabel};
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{lock, module_impl, spawn};

pub use config::VolumeModule;
use config::VolumeProfile;

#[derive(Debug, Clone)]
pub enum Update {
    SinkChange(String),
    SinkVolume(String, f64),
    SinkMute(String, bool),

    SourceChange(String),
    SourceVolume(String, f64),
    SourceMute(String, bool),

    InputVolume(u32, f64),
    InputMute(u32, bool),

    OutputVolume(u32, f64),
    OutputMute(u32, bool),
}

enum BarUiUpdate {
    Sink { muted: bool, description: String },
    Source { muted: bool, description: String },
}

struct BtnMuteUiUpdate {
    muted: bool,
    button: Option<ToggleButton>,
}

impl BtnMuteUiUpdate {
    fn muted(muted: bool) -> Self {
        Self {
            muted,
            button: None,
        }
    }
    fn muted_with(muted: bool, button: ToggleButton) -> Self {
        Self {
            muted,
            button: Some(button),
        }
    }
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

impl VolumeModule {
    fn overflow_label(&self, name: &str, css_class: &str) -> OverflowLabel {
        let label = OverflowLabel::new(Label::new(None), self.truncate, self.marquee.clone());
        label.label().add_css_class(css_class);
        label.set_label_escaped(name);
        label
    }

    fn new_slider(&self) -> Scale {
        let slider = match self.source_slider_orientation {
            ModuleOrientation::Horizontal => Scale::builder()
                .orientation(Orientation::Horizontal)
                .build(),
            ModuleOrientation::Vertical => Scale::builder()
                .orientation(Orientation::Vertical)
                .height_request(100)
                .inverted(true)
                .build(),
        };
        slider.set_range(0.0, self.max_volume);
        slider.set_value(50.0);
        slider
    }

    fn select_notify<F>(&self, selector: &DropDown, tx: Sender<Update>, func: F)
    where
        F: Fn(String) -> Update + 'static,
    {
        selector.connect_selected_notify(move |selector| {
            if let Some(item) = selector.selected_item().and_downcast_ref::<DropdownItem>() {
                tx.send_spawn(func(item.key()));
            }
        });
    }

    fn slider_notify<F>(&self, scale: Scale, selector: DropDown, tx: Sender<Update>, func: F)
    where
        F: Fn(String, f64) -> Update + 'static,
    {
        let max_volume = self.max_volume;
        scale.connect_value_changed(move |scale| {
            if scale.has_css_class("dragging")
                && let Some(item) = selector.selected_item().and_downcast_ref::<DropdownItem>()
            {
                // GTK will send values outside min/max range
                let val = scale.value().clamp(0.0, max_volume);
                tx.send_spawn(func(item.key(), val));
            }
        });
    }

    fn button_notify<F>(&self, btn: &ToggleButton, selector: DropDown, tx: Sender<Update>, func: F)
    where
        F: Fn(String, bool) -> Update + 'static,
    {
        btn.connect_toggled(move |btn| {
            if let Some(item) = selector.selected_item().and_downcast_ref::<DropdownItem>() {
                let muted = btn.is_active();
                tx.send_spawn(func(item.key(), muted));
            }
        });
    }

    fn simple_slider_notify<F>(&self, scale: &Scale, tx: Sender<Update>, i: u32, func: F)
    where
        F: Fn(u32, f64) -> Update + 'static,
    {
        let max_volume = self.max_volume;
        scale.connect_value_changed(move |scale| {
            if scale.has_css_class("dragging") {
                // GTK will send values outside min/max range
                let val = scale.value().clamp(0.0, max_volume);
                tx.send_spawn(func(i, val));
            }
        });
    }

    fn simple_button_notify<F>(&self, btn: &ToggleButton, tx: Sender<Update>, i: u32, func: F)
    where
        F: Fn(u32, bool) -> Update + 'static,
    {
        btn.connect_toggled(move |btn| {
            let muted = btn.is_active();
            tx.send_spawn(func(i, muted));
        });
    }
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

                trace!("initial sinks: {sinks:?}");

                let sources = {
                    let sources = client.sources();
                    let sources = lock!(sources);
                    sources.iter().cloned().collect::<Vec<_>>()
                };

                trace!("initial sources: {sources:?}");

                let inputs = {
                    let inputs = client.sink_inputs();
                    let inputs = lock!(inputs);
                    inputs.iter().cloned().collect::<Vec<_>>()
                };

                trace!("initial inputs: {inputs:?}");

                let outputs = {
                    let outputs = client.source_outputs();
                    let outputs = lock!(outputs);
                    outputs.iter().cloned().collect::<Vec<_>>()
                };

                trace!("initial outputs: {outputs:?}");

                for sink in sinks {
                    tx.send_update(Event::AddSink(sink)).await;
                }
                for source in sources {
                    tx.send_update(Event::AddSource(source)).await;
                }
                for input in inputs {
                    tx.send_update(Event::AddInput(input)).await;
                }
                for output in outputs {
                    tx.send_update(Event::AddOutput(output)).await;
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
                    Update::SourceChange(name) => client.set_default_source(&name),
                    Update::SourceVolume(name, volume) => client.set_source_volume(&name, volume),
                    Update::SourceMute(name, muted) => client.set_source_muted(&name, muted),
                    Update::InputVolume(index, volume) => client.set_input_volume(index, volume),
                    Update::InputMute(index, muted) => client.set_input_muted(index, muted),
                    Update::OutputVolume(index, volume) => client.set_output_volume(index, volume),
                    Update::OutputMute(index, muted) => client.set_output_muted(index, muted),
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
        let sink_label = Label::builder()
            .use_markup(true)
            .justify(self.layout.justify.into())
            .css_classes(["sink"])
            .build();

        let source_label = Label::builder()
            .use_markup(true)
            .justify(self.layout.justify.into())
            .css_classes(["source"])
            .build();

        let container = gtk::Box::new(Orientation::Horizontal, 5);
        container.append(&sink_label);
        container.append(&source_label);

        let button = Button::new();
        button.set_child(Some(&container));
        {
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
            });
        }

        let rx = context.subscribe();
        let mut manager = {
            let format = self.format.clone();
            let mute_format = self.mute_format.clone();

            // attach to button as we want class there
            self.profiles.attach(
                &button,
                move |_, event: ProfileUpdateEvent<f64, VolumeProfile, BarUiUpdate>| {
                    let icons = &event.profile.icons;
                    let (button_label, fmt, icon, desc) = match event.data {
                        BarUiUpdate::Sink { muted, description } => {
                            let (fmt, icon) = if muted {
                                sink_label.add_css_class("muted");
                                (&mute_format, &icons.muted)
                            } else {
                                sink_label.remove_css_class("muted");
                                (&format, &icons.volume)
                            };
                            (&sink_label, fmt, icon, description)
                        }
                        BarUiUpdate::Source { muted, description } => {
                            let (fmt, icon) = if muted {
                                source_label.add_css_class("muted");
                                (&mute_format, &icons.mic_muted)
                            } else {
                                source_label.remove_css_class("muted");
                                (&format, &icons.mic_volume)
                            };
                            (&source_label, fmt, icon, description)
                        }
                    };

                    let label = fmt
                        .replace("{icon}", icon)
                        .replace("{percentage}", &event.state.to_string())
                        .replace("{name}", &desc);
                    button_label.set_label_escaped(&label);
                },
            )
        };

        let show_monitors = self.show_monitors;
        rx.recv_glib((), move |(), event| match event {
            Event::AddSink(sink) | Event::UpdateSink(sink) if sink.active => {
                manager.update(
                    sink.volume.percent(),
                    BarUiUpdate::Sink {
                        muted: sink.muted,
                        description: sink.description,
                    },
                );
            }
            Event::AddSource(source) | Event::UpdateSource(source)
                if source.active && (!source.monitor || show_monitors) =>
            {
                manager.update(
                    source.volume.percent(),
                    BarUiUpdate::Source {
                        muted: source.muted,
                        description: source.description,
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
        let container = gtk::Box::new(self.popup_orientation.into(), 10);

        let sink_container = gtk::Box::new(Orientation::Vertical, 5);
        sink_container.add_css_class("sink-box");

        let source_container = gtk::Box::new(Orientation::Vertical, 5);
        source_container.add_css_class("source-box");

        let device_container = gtk::Box::new(Orientation::Vertical, 5);
        device_container.add_css_class("device-box");
        device_container.append(&sink_container);
        device_container.append(&source_container);

        let sink_input_container = gtk::Box::new(Orientation::Vertical, 5);
        sink_input_container.add_css_class("sink-input-box");

        let source_output_container = gtk::Box::new(Orientation::Vertical, 5);
        source_output_container.add_css_class("source-output-box");

        let app_container = gtk::Box::new(Orientation::Vertical, 5);
        app_container.add_css_class("apps-box");
        app_container.append(&input_container);
        app_container.append(&output_container);

        container.append(&device_container);
        container.append(&app_container);

        let sink_options = gio::ListStore::new::<DropdownItem>();
        let source_options = gio::ListStore::new::<DropdownItem>();
        let factory = SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = Label::new(None);
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&label));
        });

        let truncate = self.truncate;
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
            if let Some(truncate) = truncate {
                label.truncate(truncate);
            }
        });

        let sink_selector = DropDown::new(Some(sink_options.clone()), None::<Expression>);
        sink_selector.set_factory(Some(&factory));
        sink_selector.add_css_class("device-selector");
        sink_selector.add_css_class("sink-selector");
        {
            let tx = context.controller_tx.clone();
            self.select_notify(&sink_selector, tx, Update::SinkChange);
        }
        sink_container.append(&sink_selector);

        let source_selector = DropDown::new(Some(source_options.clone()), None::<Expression>);
        source_selector.set_factory(Some(&factory));
        source_selector.add_css_class("device-selector");
        source_selector.add_css_class("source-selector");
        {
            let tx = context.controller_tx.clone();
            self.select_notify(&source_selector, tx, Update::SourceChange);
        }
        source_container.append(&source_selector);

        let sink_slider = self.new_slider();
        sink_slider.add_css_class("slider");
        sink_slider.add_css_class("sink-slider");
        sink_container.append(&sink_slider);
        {
            let tx = context.controller_tx.clone();
            let select = sink_selector.clone();
            let scale = sink_slider.clone();
            self.slider_notify(scale, select, tx, Update::SinkVolume);
        }

        let source_slider = self.new_slider();
        source_slider.add_css_class("slider");
        source_slider.add_css_class("source-slider");
        source_container.append(&source_slider);
        {
            let tx = context.controller_tx.clone();
            let select = source_selector.clone();
            let scale = source_slider.clone();
            self.slider_notify(scale, select, tx, Update::SourceVolume);
        }

        let sink_mute = ToggleButton::new();
        sink_mute.add_css_class("btn-mute");
        sink_mute.add_css_class("sink-mute");
        sink_container.append(&sink_mute);
        {
            let tx = context.controller_tx.clone();
            let select = sink_selector.clone();
            self.button_notify(&sink_mute, select, tx, Update::SinkMute);
        }

        let source_mute = ToggleButton::new();
        source_mute.add_css_class("btn-mute");
        source_mute.add_css_class("source-mute");
        source_container.append(&source_mute);
        {
            let tx = context.controller_tx.clone();
            let selector = source_selector.clone();
            self.button_notify(&source_mute, selector, tx, Update::SourceMute);
        }

        let mut sink_manager = self.profiles.attach(
            &sink_mute,
            move |btn_mute, event: ProfileUpdateEvent<f64, VolumeProfile, BtnMuteUiUpdate>| {
                let btn = event.data.button.as_ref().unwrap_or(btn_mute);
                btn.set_active(event.data.muted);

                let icons = &event.profile.icons;
                btn.set_label(if event.data.muted {
                    &icons.muted
                } else {
                    &icons.volume
                });
            },
        );
        let mut source_manager = self.profiles.attach(
            &source_mute,
            move |btn_mute, event: ProfileUpdateEvent<f64, VolumeProfile, BtnMuteUiUpdate>| {
                let btn = event.data.button.as_ref().unwrap_or(btn_mute);
                btn.set_active(event.data.muted);

                let icons = &event.profile.icons;
                btn.set_label(if event.data.muted {
                    &icons.mic_muted
                } else {
                    &icons.mic_volume
                });
            },
        );

        let mut inputs = HashMap::new();
        let mut outputs = HashMap::new();
        let mut sinks = vec![];
        let mut sources = vec![];

        let show_monitors = self.show_monitors;
        context
            .subscribe()
            .recv_glib(
                &sink_input_container,
                move |input_container, event| match event {
                    Event::AddSink(info) => {
                        sink_options.append(&DropdownItem::new(&info.name, &info.description));

                        if info.active {
                            sink_selector.set_selected(sinks.len() as u32);
                            sink_slider.set_value(info.volume.percent());

                            sink_manager
                                .update(info.volume.percent(), BtnMuteUiUpdate::muted(info.muted));
                        }

                        sinks.push(info);
                    }
                    Event::AddSource(info) => {
                        if !info.monitor || show_monitors {
                            source_options
                                .append(&DropdownItem::new(&info.name, &info.description));

                            if info.active {
                                source_selector.set_selected(sources.len() as u32);
                                source_slider.set_value(info.volume.percent());

                                source_manager.update(
                                    info.volume.percent(),
                                    BtnMuteUiUpdate::muted(info.muted),
                                );
                            }

                            sources.push(info);
                        }
                    }
                    Event::UpdateSink(info) => {
                        if info.active
                            && let Some(pos) = sinks.iter().position(|s| s.name == info.name)
                        {
                            sink_selector.set_selected(pos as u32);
                            if !sink_slider.has_css_class("dragging") {
                                sink_slider.set_value(info.volume.percent());
                            }

                            sink_manager
                                .update(info.volume.percent(), BtnMuteUiUpdate::muted(info.muted));
                        }
                    }
                    Event::UpdateSource(info) => {
                        if info.active
                            && let Some(pos) = sources.iter().position(|s| s.name == info.name)
                            && (!info.monitor || show_monitors)
                        {
                            source_selector.set_selected(pos as u32);
                            if !source_slider.has_css_class("dragging") {
                                source_slider.set_value(info.volume.percent());
                            }

                            source_manager
                                .update(info.volume.percent(), BtnMuteUiUpdate::muted(info.muted));
                        }
                    }
                    Event::RemoveSink(name) => {
                        if let Some(pos) = sinks.iter().position(|s| s.name == name) {
                            sink_options.remove(pos as u32);
                            sinks.remove(pos);
                        }
                    }
                    Event::RemoveSource(name) => {
                        if let Some(pos) = sources.iter().position(|s| s.name == name) {
                            source_options.remove(pos as u32);
                            sources.remove(pos);
                        }
                    }
                    Event::AddInput(info) => {
                        let index = info.index;

                        let item_container = gtk::Box::new(Orientation::Vertical, 0);
                        item_container.add_css_class("app-box");
                        item_container.add_css_class("input-box");

                        let title_label = self.overflow_label(&info.name, "title");
                        item_container.append(title_label.widget());

                        let tx = context.controller_tx.clone();
                        let slider = Scale::builder().sensitive(info.can_set_volume).build();
                        slider.set_range(0.0, self.max_volume);
                        slider.set_value(info.volume.percent());
                        slider.add_css_class("slider");
                        self.simple_slider_notify(&slider, tx, index, Update::InputVolume);

                        let tx = context.controller_tx.clone();
                        let btn_mute = ToggleButton::new();
                        btn_mute.add_css_class("btn-mute");
                        btn_mute.add_css_class("sink-mute");
                        self.simple_button_notify(&btn_mute, tx, index, Update::InputMute);

                        item_container.append(&slider);
                        item_container.append(&btn_mute);
                        input_container.append(&item_container);

                        sink_manager.update(
                            info.volume.percent(),
                            BtnMuteUiUpdate::muted_with(info.muted, btn_mute.clone()),
                        );

                        inputs.insert(
                            info.index,
                            VolumeUi {
                                container: item_container,
                                button: btn_mute,
                                title_label,
                                slider,
                                label_raw: info.name.clone(),
                            },
                        );
                    }
                    Event::AddOutput(info) => {
                        let index = info.index;

                        let item_container = gtk::Box::new(Orientation::Vertical, 0);
                        item_container.add_css_class("app-box");
                        item_container.add_css_class("output-box");

                        let title_label = self.overflow_label(&info.name, "title");
                        item_container.append(title_label.widget());

                        let tx = context.controller_tx.clone();
                        let slider = Scale::builder().sensitive(info.can_set_volume).build();
                        slider.set_range(0.0, self.max_volume);
                        slider.set_value(info.volume.percent());
                        slider.add_css_class("slider");
                        self.simple_slider_notify(&slider, tx, index, Update::OutputVolume);

                        let tx = context.controller_tx.clone();
                        let btn_mute = ToggleButton::new();
                        btn_mute.add_css_class("btn-mute");
                        btn_mute.add_css_class("source-mute");
                        self.simple_button_notify(&btn_mute, tx, index, Update::OutputMute);

                        item_container.append(&slider);
                        item_container.append(&btn_mute);
                        source_output_container.append(&item_container);

                        source_manager.update(
                            info.volume.percent(),
                            BtnMuteUiUpdate::muted_with(info.muted, btn_mute.clone()),
                        );

                        outputs.insert(
                            info.index,
                            VolumeUi {
                                container: item_container,
                                button: btn_mute,
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
                            sink_manager.update(
                                info.volume.percent(),
                                BtnMuteUiUpdate::muted_with(info.muted, ui.button.clone()),
                            );
                        }
                    }
                    Event::UpdateOutput(info) => {
                        if let Some(ui) = outputs.get_mut(&info.index) {
                            if ui.label_raw != info.name {
                                ui.title_label.set_label_escaped(&info.name);
                                ui.label_raw = info.name.clone();
                            }

                            if !ui.slider.has_css_class("dragging") {
                                ui.slider.set_value(info.volume.percent());
                            }

                            ui.slider.set_sensitive(info.can_set_volume);
                            source_manager.update(
                                info.volume.percent(),
                                BtnMuteUiUpdate::muted_with(info.muted, ui.button.clone()),
                            );
                        }
                    }
                    Event::RemoveInput(index) => {
                        if let Some(ui) = inputs.remove(&index) {
                            input_container.remove(&ui.container);
                        }
                    }
                    Event::RemoveOutput(index) => {
                        if let Some(ui) = outputs.remove(&index) {
                            source_output_container.remove(&ui.container);
                        }
                    }
                },
            );

        Some(container)
    }
}

struct VolumeUi {
    container: gtk::Box,
    title_label: OverflowLabel,
    slider: Scale,
    button: ToggleButton,
    // Store original (unformatted) title to detect change when marquee is enabled
    label_raw: String,
}
