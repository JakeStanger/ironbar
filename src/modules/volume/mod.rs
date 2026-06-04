mod config;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use glib::subclass::prelude::*;
use glib::{Object, Properties};
use gtk::prelude::*;
use gtk::{
    Button, DropDown, Expression, Label, ListItem, Orientation, Scale, SignalListItemFactory,
    ToggleButton, gio,
};
use tokio::sync::mpsc::{self};
use tracing::trace;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::volume::{self, Event, Sink, SinkInput, Source, SourceOutput, VolumeLevels};
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
    Sink {
        muted: bool,
        description: String,
        show: bool,
    },
    Source {
        muted: bool,
        description: String,
        show: bool,
    },
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

#[derive(Debug, Clone)]
struct Device {
    name: String,
    description: String,
    volume: VolumeLevels,
    muted: bool,
}

impl From<Source> for Device {
    fn from(source: Source) -> Device {
        Device {
            name: source.name,
            description: source.description,
            volume: source.volume,
            muted: source.muted,
        }
    }
}

impl From<Sink> for Device {
    fn from(sink: Sink) -> Device {
        Device {
            name: sink.name,
            description: sink.description,
            volume: sink.volume,
            muted: sink.muted,
        }
    }
}

#[derive(Debug, Clone)]
struct Stream {
    index: u32,
    name: String,
    volume: VolumeLevels,
    muted: bool,
    can_set_volume: bool,
}

impl From<SourceOutput> for Stream {
    fn from(source_output: SourceOutput) -> Stream {
        Stream {
            index: source_output.index,
            name: source_output.name,
            volume: source_output.volume,
            muted: source_output.muted,
            can_set_volume: source_output.can_set_volume,
        }
    }
}

impl From<SinkInput> for Stream {
    fn from(sink_input: SinkInput) -> Stream {
        Stream {
            index: sink_input.index,
            name: sink_input.name,
            volume: sink_input.volume,
            muted: sink_input.muted,
            can_set_volume: sink_input.can_set_volume,
        }
    }
}

struct DeviceUi {
    config: VolumeModule,
    default_device_name: Option<String>,
    container: gtk::Box,
    selector: DropDown,
    slider: Scale,
    options: gio::ListStore,
    no_device: DropdownItem,
    btn_mute: ToggleButton,
    profile_update: Box<dyn FnMut(f64, BtnMuteUiUpdate)>,
}

// This struct avoids the too many arguments clippy warning in DeviceUi::new
struct DeviceUiCallbacks {
    select_notify_fn: fn(String) -> Update,
    slider_notify_fn: fn(String, f64) -> Update,
    button_notify_fn: fn(String, bool) -> Update,
    profile_update_fn: fn(&ToggleButton, ProfileUpdateEvent<f64, VolumeProfile, BtnMuteUiUpdate>),
}

impl DeviceUi {
    fn new(
        config: VolumeModule,
        context: &WidgetContext<Event, Update>,
        ignore_selected: &Rc<Cell<bool>>,
        no_device: DropdownItem,
        callbacks: DeviceUiCallbacks,
    ) -> Self {
        let container = gtk::Box::new(Orientation::Vertical, 5);

        let options = gio::ListStore::new::<DropdownItem>();
        options.append(&no_device);

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

            label.set_label(&dropdown_item.value().clone());
            if let Some(truncate) = config.truncate_popup {
                label.truncate(truncate);
            }
        });

        let selector = DropDown::new(Some(options.clone()), None::<Expression>);
        selector.set_factory(Some(&factory));
        selector.add_css_class("device-selector");
        {
            let tx = context.controller_tx.clone();
            let ignore_selected = ignore_selected.clone();
            selector.connect_selected_item_notify(move |selector| {
                if !ignore_selected.get()
                    && let Some(item) = selector.selected_item().and_downcast_ref::<DropdownItem>()
                {
                    tx.send_spawn((callbacks.select_notify_fn)(item.key()));
                }
            });
        }
        container.append(&selector);

        let slider = match config.source_slider_orientation {
            ModuleOrientation::Horizontal => Scale::builder()
                .orientation(Orientation::Horizontal)
                .build(),
            ModuleOrientation::Vertical => Scale::builder()
                .orientation(Orientation::Vertical)
                .height_request(100)
                .inverted(true)
                .build(),
        };
        slider.set_range(0.0, config.max_volume);
        slider.add_css_class("slider");
        container.append(&slider);
        {
            let tx = context.controller_tx.clone();
            let max_volume = config.max_volume;
            let selector = selector.clone();
            slider.connect_value_changed(move |slider| {
                if slider.has_css_class("dragging")
                    && let Some(item) = selector.selected_item().and_downcast_ref::<DropdownItem>()
                {
                    // GTK will send values outside min/max range
                    let val = slider.value().clamp(0.0, max_volume);
                    tx.send_spawn((callbacks.slider_notify_fn)(item.key(), val));
                }
            });
        }

        let btn_mute = ToggleButton::new();
        btn_mute.add_css_class("btn-mute");
        container.append(&btn_mute);
        {
            let tx = context.controller_tx.clone();
            let selector = selector.clone();
            btn_mute.connect_toggled(move |btn_mute| {
                if let Some(item) = selector.selected_item().and_downcast_ref::<DropdownItem>() {
                    let muted = btn_mute.is_active();
                    tx.send_spawn((callbacks.button_notify_fn)(item.key(), muted));
                };
            });
        }

        let mut manager = config
            .profiles
            .attach(&btn_mute, callbacks.profile_update_fn);
        let ui = Self {
            config,
            default_device_name: None,
            container,
            selector,
            slider,
            options,
            no_device,
            btn_mute,
            profile_update: Box::new(move |v, d| manager.update(v, d)),
        };
        ui.set_no_device();
        ui
    }

    fn set_default_device_name(&mut self, default_device_name: String) {
        self.default_device_name = Some(default_device_name);
    }

    fn add_device(&mut self, device: Device, show: bool) {
        if show {
            self.options
                .append(&DropdownItem::new(&device.name, &device.description));
        }

        if !self.is_default(&device.name) {
            return;
        }

        if show {
            self.set_device(device, self.options.n_items() - 1);
        } else {
            self.set_no_device();
        }
    }

    fn update_device(&mut self, device: Device) {
        if !self.is_default(&device.name) {
            return;
        }

        if let Some(pos) = self.get_device_position(&device.name) {
            self.set_device(device, pos);
        } else {
            self.set_no_device();
        }
    }

    fn remove_device(&mut self, device_name: &str) {
        if let Some(pos) = self.get_device_position(device_name) {
            self.options.remove(pos);

            if self.options.n_items() == 0 {
                self.set_no_device();
            }
        }
    }

    fn set_device(&mut self, device: Device, pos: u32) {
        self.selector.set_selected(pos);
        self.slider.set_visible(true);
        if !self.slider.has_css_class("dragging") {
            self.slider.set_value(device.volume.percent());
        }
        self.btn_mute.set_visible(true);
        (self.profile_update)(
            device.volume.percent(),
            BtnMuteUiUpdate {
                muted: device.muted,
            },
        );

        if let Some(no_device_pos) = self.options.find(&self.no_device) {
            self.options.remove(no_device_pos);
        }
    }

    fn set_no_device(&self) {
        let no_device_pos = if let Some(pos) = self.options.find(&self.no_device) {
            pos
        } else {
            self.options.insert(0, &self.no_device);
            0
        };
        self.selector.set_selected(no_device_pos);
        self.slider.set_visible(false);
        self.btn_mute.set_visible(false);
    }

    fn is_default(&self, device_name: &str) -> bool {
        self.default_device_name.as_deref() == Some(device_name)
    }

    fn get_device_position(&self, device_name: &str) -> Option<u32> {
        self.options
            .iter::<DropdownItem>()
            .position(|s| s.is_ok_and(|s| s.key() == *device_name))
            .map(|pos| pos as u32)
    }
}

struct SinkUi {
    inner: DeviceUi,
}

impl SinkUi {
    fn new(
        config: &VolumeModule,
        context: &WidgetContext<Event, Update>,
        ignore_selected: &Rc<Cell<bool>>,
    ) -> Self {
        let inner = DeviceUi::new(
            config.clone(),
            context,
            ignore_selected,
            DropdownItem::new("ironbar_no_sink", "No sink available"),
            DeviceUiCallbacks {
                select_notify_fn: Update::SinkChange,
                slider_notify_fn: Update::SinkVolume,
                button_notify_fn: Update::SinkMute,
                profile_update_fn: update_sink_mute,
            },
        );
        inner.container.add_css_class("sink-box");
        inner.selector.add_css_class("sink-selector");
        inner.slider.add_css_class("sink-slider");
        inner.btn_mute.add_css_class("sink-mute");
        Self { inner }
    }

    fn add_sink(&mut self, sink: Sink) {
        self.inner.add_device(sink.into(), true);
    }

    fn update_sink(&mut self, sink: Sink) {
        self.inner.update_device(sink.into());
    }
}

struct SourceUi {
    inner: DeviceUi,
}

impl SourceUi {
    fn new(
        config: &VolumeModule,
        context: &WidgetContext<Event, Update>,
        ignore_selected: &Rc<Cell<bool>>,
    ) -> Self {
        let inner = DeviceUi::new(
            config.clone(),
            context,
            ignore_selected,
            DropdownItem::new("ironbar_no_source", "No source available"),
            DeviceUiCallbacks {
                select_notify_fn: Update::SourceChange,
                slider_notify_fn: Update::SourceVolume,
                button_notify_fn: Update::SourceMute,
                profile_update_fn: update_source_mute,
            },
        );
        inner.container.add_css_class("source-box");
        inner.selector.add_css_class("source-selector");
        inner.slider.add_css_class("source-slider");
        inner.btn_mute.add_css_class("source-mute");
        Self { inner }
    }

    fn add_source(&mut self, source: Source) {
        let show = !source.monitor || self.inner.config.show_monitors;
        self.inner.add_device(source.into(), show);
    }

    fn update_source(&mut self, source: Source) {
        self.inner.update_device(source.into());
    }
}

struct StreamUi {
    container: gtk::Box,
    title_label: OverflowLabel,
    slider: Scale,
    btn_mute: ToggleButton,
    // Store original (unformatted) title to detect change when marquee is enabled
    label_raw: String,
    profile_update: Box<dyn FnMut(f64, BtnMuteUiUpdate)>,
}

impl StreamUi {
    fn new(
        stream: Stream,
        config: &VolumeModule,
        context: &WidgetContext<Event, Update>,
        slider_notify_fn: fn(u32, f64) -> Update,
        button_notify_fn: fn(u32, bool) -> Update,
        profile_update_fn: fn(
            &ToggleButton,
            ProfileUpdateEvent<f64, VolumeProfile, BtnMuteUiUpdate>,
        ),
    ) -> Self {
        let index = stream.index;

        let container = gtk::Box::new(Orientation::Vertical, 0);
        container.add_css_class("app-box");

        let title_label =
            OverflowLabel::new(Label::new(None), config.truncate_popup, &config.marquee);
        title_label.label().add_css_class("title");
        container.append(title_label.widget());

        let tx = context.controller_tx.clone();
        let slider = Scale::builder().sensitive(stream.can_set_volume).build();
        let max_volume = config.max_volume;
        slider.set_range(0.0, max_volume);
        slider.add_css_class("slider");
        slider.connect_value_changed(move |slider| {
            if slider.has_css_class("dragging") {
                // GTK will send values outside min/max range
                let val = slider.value().clamp(0.0, max_volume);
                tx.send_spawn(slider_notify_fn(index, val));
            }
        });

        let tx = context.controller_tx.clone();
        let btn_mute = ToggleButton::new();
        btn_mute.add_css_class("btn-mute");
        btn_mute.connect_toggled(move |btn| {
            let muted = btn.is_active();
            tx.send_spawn(button_notify_fn(index, muted));
        });

        container.append(&slider);
        container.append(&btn_mute);

        let mut manager = config.profiles.attach(&btn_mute, profile_update_fn);
        let mut ui = Self {
            container,
            title_label,
            slider,
            btn_mute,
            label_raw: String::default(),
            profile_update: Box::new(move |v, d| manager.update(v, d)),
        };
        ui.update_stream(stream);
        ui
    }

    fn update_stream(&mut self, stream: Stream) {
        if self.label_raw != stream.name {
            self.title_label.set_label_escaped(&stream.name);
            self.label_raw = stream.name;
        }

        if !self.slider.has_css_class("dragging") {
            self.slider.set_value(stream.volume.percent());
        }

        self.slider.set_sensitive(stream.can_set_volume);
        (self.profile_update)(
            stream.volume.percent(),
            BtnMuteUiUpdate {
                muted: stream.muted,
            },
        );
    }
}

struct SinkInputUi {
    inner: StreamUi,
}

impl SinkInputUi {
    fn new(
        sink_input: SinkInput,
        config: &VolumeModule,
        context: &WidgetContext<Event, Update>,
    ) -> Self {
        let inner = StreamUi::new(
            sink_input.into(),
            config,
            context,
            Update::InputVolume,
            Update::InputMute,
            update_sink_mute,
        );
        inner.container.add_css_class("input-box");
        inner.btn_mute.add_css_class("sink-mute");
        Self { inner }
    }

    fn update_sink_input(&mut self, sink_input: SinkInput) {
        self.inner.update_stream(sink_input.into());
    }
}

struct SourceOutputUi {
    inner: StreamUi,
}

impl SourceOutputUi {
    fn new(
        source_output: SourceOutput,
        config: &VolumeModule,
        context: &WidgetContext<Event, Update>,
    ) -> Self {
        let inner = StreamUi::new(
            source_output.into(),
            config,
            context,
            Update::OutputVolume,
            Update::OutputMute,
            update_source_mute,
        );
        inner.container.add_css_class("output-box");
        inner.btn_mute.add_css_class("source-mute");
        Self { inner }
    }

    fn update_source_output(&mut self, source_output: SourceOutput) {
        self.inner.update_stream(source_output.into());
    }
}

fn update_sink_mute(
    btn_mute: &ToggleButton,
    event: ProfileUpdateEvent<f64, VolumeProfile, BtnMuteUiUpdate>,
) {
    btn_mute.set_active(event.data.muted);

    let icons = &event.profile.icons;
    btn_mute.set_label(if event.data.muted {
        &icons.muted
    } else {
        &icons.volume
    });
}

fn update_source_mute(
    btn_mute: &ToggleButton,
    event: ProfileUpdateEvent<f64, VolumeProfile, BtnMuteUiUpdate>,
) {
    btn_mute.set_active(event.data.muted);

    let icons = &event.profile.icons;
    btn_mute.set_label(if event.data.muted {
        &icons.mic_muted
    } else {
        &icons.mic_volume
    });
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
                let default_sink = client.default_sink();
                let default_source = client.default_source();

                trace!("default sink: {default_sink:?}");
                trace!("default source: {default_source:?}");

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

                if let Some(default_sink) = default_sink {
                    tx.send_update(Event::SetDefaultSink(default_sink)).await;
                }

                if let Some(default_source) = default_source {
                    tx.send_update(Event::SetDefaultSource(default_source))
                        .await;
                }

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
                    let (button_label, fmt, icon, desc, show) = match event.data {
                        BarUiUpdate::Sink {
                            muted,
                            description,
                            show,
                        } => {
                            let (fmt, icon) = if muted {
                                sink_label.add_css_class("muted");
                                (&mute_format, &icons.muted)
                            } else {
                                sink_label.remove_css_class("muted");
                                (&format, &icons.volume)
                            };
                            (&sink_label, fmt, icon, description, show)
                        }
                        BarUiUpdate::Source {
                            muted,
                            description,
                            show,
                        } => {
                            let (fmt, icon) = if muted {
                                source_label.add_css_class("muted");
                                (&mute_format, &icons.mic_muted)
                            } else {
                                source_label.remove_css_class("muted");
                                (&format, &icons.mic_volume)
                            };
                            (&source_label, fmt, icon, description, show)
                        }
                    };

                    let label = fmt
                        .replace("{icon}", icon)
                        .replace("{percentage}", &event.state.to_string())
                        .replace("{name}", &desc);
                    button_label.set_label_escaped(&label);

                    if let Some(truncate) = self.truncate {
                        button_label.truncate(truncate);
                    }

                    if show {
                        if button_label.parent().is_none() {
                            container.append(button_label);
                        }
                    } else {
                        if button_label.parent().is_some() {
                            container.remove(button_label);
                        }
                    }
                },
            )
        };

        let show_monitors = self.show_monitors;

        let mut default_sink = None;
        let mut default_source = None;

        rx.recv_glib((), move |(), event| match event {
            Event::SetDefaultSink(name) => default_sink = Some(name),
            Event::SetDefaultSource(name) => default_source = Some(name),
            Event::AddSink(sink) | Event::UpdateSink(sink)
                if default_sink
                    .as_deref()
                    .is_some_and(|name| name == sink.name) =>
            {
                manager.update(
                    sink.volume.percent(),
                    BarUiUpdate::Sink {
                        muted: sink.muted,
                        description: sink.description,
                        show: self.show_sinks,
                    },
                );
            }
            Event::AddSource(source) | Event::UpdateSource(source)
                if default_source
                    .as_deref()
                    .is_some_and(|name| name == source.name) =>
            {
                manager.update(
                    source.volume.percent(),
                    BarUiUpdate::Source {
                        muted: source.muted,
                        description: source.description,
                        show: self.show_sources && (!source.monitor || show_monitors),
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

        let device_container = gtk::Box::new(Orientation::Vertical, 5);
        device_container.add_css_class("device-box");

        let sink_input_container = gtk::Box::new(Orientation::Vertical, 5);
        sink_input_container.add_css_class("sink-input-box");

        let source_output_container = gtk::Box::new(Orientation::Vertical, 5);
        source_output_container.add_css_class("source-output-box");

        let app_container = gtk::Box::new(Orientation::Vertical, 5);
        app_container.add_css_class("apps-box");

        if self.show_sinks {
            app_container.append(&sink_input_container);
        }

        if self.show_sources {
            app_container.append(&source_output_container);
        }

        container.append(&device_container);
        container.append(&app_container);

        let mut inputs = HashMap::new();
        let mut outputs = HashMap::new();

        let ignore_selected = Rc::new(Cell::new(false));
        let mut sink_ui = SinkUi::new(&self, &context, &ignore_selected);
        let mut source_ui = SourceUi::new(&self, &context, &ignore_selected);

        if self.show_sinks {
            device_container.append(&sink_ui.inner.container);
        }

        if self.show_sources {
            device_container.append(&source_ui.inner.container);
        }

        context
            .subscribe()
            .recv_glib(&sink_input_container, move |input_container, event| {
                // Ignore selected sink and source notifications caused by programmatic changes
                let _ignore_guard = IgnoreGuard::new(&ignore_selected);
                match event {
                    Event::SetDefaultSink(name) => sink_ui.inner.set_default_device_name(name),
                    Event::SetDefaultSource(name) => source_ui.inner.set_default_device_name(name),
                    Event::AddSink(info) => sink_ui.add_sink(info),
                    Event::AddSource(info) => source_ui.add_source(info),
                    Event::UpdateSink(info) => sink_ui.update_sink(info),
                    Event::UpdateSource(info) => source_ui.update_source(info),
                    Event::RemoveSink(name) => sink_ui.inner.remove_device(&name),
                    Event::RemoveSource(name) => source_ui.inner.remove_device(&name),
                    Event::AddInput(info) => {
                        let index = info.index;
                        let input_ui = SinkInputUi::new(info, &self, &context);
                        input_container.append(&input_ui.inner.container);
                        inputs.insert(index, input_ui);
                    }
                    Event::AddOutput(info) => {
                        let index = info.index;
                        let output_ui = SourceOutputUi::new(info, &self, &context);
                        source_output_container.append(&output_ui.inner.container);
                        outputs.insert(index, output_ui);
                    }
                    Event::UpdateInput(info) => {
                        if let Some(ui) = inputs.get_mut(&info.index) {
                            ui.update_sink_input(info);
                        }
                    }
                    Event::UpdateOutput(info) => {
                        if let Some(ui) = outputs.get_mut(&info.index) {
                            ui.update_source_output(info);
                        }
                    }
                    Event::RemoveInput(index) => {
                        if let Some(ui) = inputs.remove(&index) {
                            input_container.remove(&ui.inner.container);
                        }
                    }
                    Event::RemoveOutput(index) => {
                        if let Some(ui) = outputs.remove(&index) {
                            source_output_container.remove(&ui.inner.container);
                        }
                    }
                };
            });

        Some(container)
    }
}

struct IgnoreGuard<'a> {
    ignore: &'a Cell<bool>,
}

impl<'a> IgnoreGuard<'a> {
    fn new(ignore: &'a Cell<bool>) -> Self {
        ignore.set(true);
        Self { ignore }
    }
}

impl<'a> Drop for IgnoreGuard<'a> {
    fn drop(&mut self) {
        self.ignore.set(false);
    }
}
