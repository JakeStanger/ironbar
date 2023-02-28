use crate::clients::clipboard::{self, ClipboardEvent};
use crate::clients::wayland::{ClipboardItem, ClipboardValue};
use crate::config::{CommonConfig, TruncateMode};
use crate::image::new_icon_button;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::popup::Popup;
use crate::try_send;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio::{Cancellable, MemoryInputStream};
use gtk::prelude::*;
use gtk::{Button, EventBox, Image, Label, Orientation, RadioButton, Widget};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{debug, error};

#[derive(Debug, Deserialize, Clone)]
pub struct ClipboardModule {
    #[serde(default = "default_icon")]
    icon: String,

    #[serde(default = "default_max_items")]
    max_items: usize,

    // -- Common --
    truncate: Option<TruncateMode>,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

fn default_icon() -> String {
    String::from("󰨸")
}

const fn default_max_items() -> usize {
    10
}

#[derive(Debug, Clone)]
pub enum ControllerEvent {
    Add(usize, Arc<ClipboardItem>),
    Remove(usize),
    Activate(usize),
    Deactivate,
}

#[derive(Debug, Clone)]
pub enum UIEvent {
    Copy(usize),
    Remove(usize),
}

impl Module<Button> for ClipboardModule {
    type SendMessage = ControllerEvent;
    type ReceiveMessage = UIEvent;

    fn name() -> &'static str {
        "clipboard"
    }

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> color_eyre::Result<()> {
        let max_items = self.max_items;

        // listen to clipboard events
        spawn(async move {
            let mut rx = {
                let client = clipboard::get_client();
                client.subscribe(max_items).await
            };

            while let Some(event) = rx.recv().await {
                match event {
                    ClipboardEvent::Add(item) => {
                        let msg = match &item.value {
                            ClipboardValue::Other => {
                                ModuleUpdateEvent::Update(ControllerEvent::Deactivate)
                            }
                            _ => ModuleUpdateEvent::Update(ControllerEvent::Add(item.id, item)),
                        };
                        try_send!(tx, msg);
                    }
                    ClipboardEvent::Remove(id) => {
                        try_send!(tx, ModuleUpdateEvent::Update(ControllerEvent::Remove(id)));
                    }
                    ClipboardEvent::Activate(id) => {
                        try_send!(tx, ModuleUpdateEvent::Update(ControllerEvent::Activate(id)));
                    }
                }
            }

            error!("Clipboard client unexpectedly closed");
        });

        // listen to ui events
        spawn(async move {
            while let Some(event) = rx.recv().await {
                let client = clipboard::get_client();
                match event {
                    UIEvent::Copy(id) => client.copy(id).await,
                    UIEvent::Remove(id) => client.remove(id),
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> color_eyre::Result<ModuleWidget<Button>> {
        let position = info.bar_position;

        let button = new_icon_button(&self.icon, info.icon_theme, 32);
        button.style_context().add_class("btn");

        button.connect_clicked(move |button| {
            let pos = Popup::button_pos(button, position.get_orientation());
            try_send!(context.tx, ModuleUpdateEvent::TogglePopup(pos));
        });

        // we need to bind to the receiver as the channel does not open
        // until the popup is first opened.
        context.widget_rx.attach(None, |_| Continue(true));

        Ok(ModuleWidget {
            widget: button,
            popup: self.into_popup(context.controller_tx, context.popup_rx, info),
        })
    }

    fn into_popup(
        self,
        tx: Sender<Self::ReceiveMessage>,
        rx: glib::Receiver<Self::SendMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        let container = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .name("popup-clipboard")
            .build();

        let entries = gtk::Box::new(Orientation::Vertical, 5);
        container.add(&entries);

        let hidden_option = RadioButton::new();
        entries.add(&hidden_option);

        let mut items = HashMap::new();

        {
            let hidden_option = hidden_option.clone();
            rx.attach(None, move |event| {
                match event {
                    ControllerEvent::Add(id, item) => {
                        debug!("Adding new value with ID {}", id);

                        let row = gtk::Box::new(Orientation::Horizontal, 0);
                        row.style_context().add_class("item");

                        let button = match &item.value {
                            ClipboardValue::Text(value) => {
                                let button = RadioButton::from_widget(&hidden_option);

                                let label = Label::new(Some(value));
                                button.add(&label);

                                if let Some(truncate) = self.truncate {
                                    truncate.truncate_label(&label);
                                }

                                button.style_context().add_class("text");
                                button
                            }
                            ClipboardValue::Image(bytes) => {
                                let stream = MemoryInputStream::from_bytes(bytes);
                                let pixbuf = Pixbuf::from_stream_at_scale(
                                    &stream,
                                    128,
                                    64,
                                    true,
                                    Some(&Cancellable::new()),
                                )
                                .expect("Failed to read Pixbuf from stream");
                                let image = Image::from_pixbuf(Some(&pixbuf));

                                let button = RadioButton::from_widget(&hidden_option);
                                button.set_image(Some(&image));
                                button.set_always_show_image(true);
                                button.style_context().add_class("image");

                                button
                            }
                            ClipboardValue::Other => unreachable!(),
                        };

                        button.style_context().add_class("btn");
                        button.set_active(true); // if just added, should be on clipboard

                        let button_wrapper = EventBox::new();
                        button_wrapper.add(&button);

                        button_wrapper.set_widget_name(&format!("copy-{id}"));
                        button_wrapper.set_above_child(true);

                        {
                            let tx = tx.clone();
                            button_wrapper.connect_button_press_event(
                                move |button_wrapper, event| {
                                    // left click
                                    if event.button() == 1 {
                                        let id = get_button_id(button_wrapper)
                                            .expect("Failed to get id from button name");

                                        debug!("Copying item with id: {id}");
                                        try_send!(tx, UIEvent::Copy(id));
                                    }

                                    Inhibit(true)
                                },
                            );
                        }

                        let remove_button = Button::with_label("x");
                        remove_button.set_widget_name(&format!("remove-{id}"));
                        remove_button.style_context().add_class("btn-remove");

                        {
                            let tx = tx.clone();
                            let entries = entries.clone();
                            let row = row.clone();

                            remove_button.connect_clicked(move |button| {
                                let id = get_button_id(button)
                                    .expect("Failed to get id from button name");

                                debug!("Removing item with id: {id}");
                                try_send!(tx, UIEvent::Remove(id));

                                entries.remove(&row);
                            });
                        }

                        row.add(&button_wrapper);
                        row.pack_end(&remove_button, false, false, 0);

                        entries.add(&row);
                        entries.reorder_child(&row, 0);
                        row.show_all();

                        items.insert(id, (row, button));
                    }
                    ControllerEvent::Remove(id) => {
                        debug!("Removing option with ID {id}");
                        let row = items.remove(&id);
                        if let Some((row, button)) = row {
                            if button.is_active() {
                                hidden_option.set_active(true);
                            }

                            entries.remove(&row);
                        }
                    }
                    ControllerEvent::Activate(id) => {
                        debug!("Activating option with ID {id}");

                        hidden_option.set_active(false);
                        let row = items.get(&id);
                        if let Some((_, button)) = row {
                            button.set_active(true);
                        }
                    }
                    ControllerEvent::Deactivate => {
                        debug!("Deactivating current option");
                        hidden_option.set_active(true);
                    }
                }

                Continue(true)
            });
        }

        container.show_all();
        hidden_option.hide();

        Some(container)
    }
}

/// Gets the ID from a widget's name.
///
/// This expects the button name to be
/// in the format `<purpose>-<id>`.
fn get_button_id<W>(button_wrapper: &W) -> Option<usize>
where
    W: IsA<Widget>,
{
    button_wrapper
        .widget_name()
        .split_once('-')
        .and_then(|(_, id)| id.parse().ok())
}
