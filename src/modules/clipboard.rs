use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::clipboard::{self, ClipboardEvent};
use crate::clients::wayland::{ClipboardItem, ClipboardValue};
use crate::config::{CommonConfig, TruncateMode};
use crate::gtk_helpers::IronbarLabelExt;
use crate::image::new_icon_button;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{module_impl, spawn};
use glib::Propagation;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio::{Cancellable, MemoryInputStream};
use gtk::prelude::*;
use gtk::{Button, EventBox, Image, Label, Orientation, RadioButton, Widget};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ClipboardModule {
    /// The icon to show on the bar widget button.
    /// Supports [image](images) icons.
    ///
    /// **Default**: `󰨸`
    #[serde(default = "default_icon")]
    icon: String,

    /// The size to render the icon at.
    /// Note this only applies to image-type icons.
    ///
    /// **Default**: `32`
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    /// The maximum number of items to keep in the history,
    /// and to show in the popup.
    ///
    /// **Default**: `10`
    #[serde(default = "default_max_items")]
    max_items: usize,

    // -- Common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    truncate: Option<TruncateMode>,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

fn default_icon() -> String {
    String::from("󰨸")
}

const fn default_icon_size() -> i32 {
    32
}

const fn default_max_items() -> usize {
    10
}

#[derive(Debug, Clone)]
pub enum ControllerEvent {
    Add(usize, ClipboardItem),
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

    module_impl!("clipboard");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> color_eyre::Result<()> {
        let max_items = self.max_items;

        let tx = context.tx.clone();
        let client = context.client::<clipboard::Client>();

        // listen to clipboard events
        spawn(async move {
            let mut rx = client.subscribe(max_items);

            while let Some(event) = rx.recv().await {
                match event {
                    ClipboardEvent::Add(item) => {
                        let msg = match item.value.as_ref() {
                            ClipboardValue::Other => ControllerEvent::Deactivate,
                            _ => ControllerEvent::Add(item.id, item),
                        };
                        tx.send_update_spawn(msg);
                    }
                    ClipboardEvent::Remove(id) => {
                        tx.send_update_spawn(ControllerEvent::Remove(id));
                    }
                    ClipboardEvent::Activate(id) => {
                        tx.send_update_spawn(ControllerEvent::Activate(id));
                    }
                }
            }

            error!("Clipboard client unexpectedly closed");
        });

        let client = context.client::<clipboard::Client>();

        // listen to ui events
        spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    UIEvent::Copy(id) => client.copy(id),
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
    ) -> color_eyre::Result<ModuleParts<Button>> {
        let button = new_icon_button(&self.icon, info.icon_theme, self.icon_size);
        button.style_context().add_class("btn");

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        let rx = context.subscribe();
        let popup = self
            .into_popup(context.controller_tx.clone(), rx, context, info)
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        tx: mpsc::Sender<Self::ReceiveMessage>,
        rx: broadcast::Receiver<Self::SendMessage>,
        _context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        let container = gtk::Box::new(Orientation::Vertical, 10);

        let entries = gtk::Box::new(Orientation::Vertical, 5);
        container.add(&entries);

        let hidden_option = RadioButton::new();
        entries.add(&hidden_option);

        let mut items = HashMap::new();

        {
            let hidden_option = hidden_option.clone();
            rx.recv_glib(move |event| {
                match event {
                    ControllerEvent::Add(id, item) => {
                        debug!("Adding new value with ID {}", id);

                        let row = gtk::Box::new(Orientation::Horizontal, 0);
                        row.style_context().add_class("item");

                        let button = match item.value.as_ref() {
                            ClipboardValue::Text(value) => {
                                let button = RadioButton::from_widget(&hidden_option);

                                let label = Label::new(Some(value));
                                button.add(&label);

                                if let Some(truncate) = self.truncate {
                                    label.truncate(truncate);
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
                                );

                                match pixbuf {
                                    Ok(pixbuf) => {
                                        let image = Image::from_pixbuf(Some(&pixbuf));

                                        let button = RadioButton::from_widget(&hidden_option);
                                        button.set_image(Some(&image));
                                        button.set_always_show_image(true);
                                        button.style_context().add_class("image");

                                        button
                                    }
                                    Err(err) => {
                                        error!("{err:?}");
                                        return;
                                    }
                                }
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
                                        tx.send_spawn(UIEvent::Copy(id));
                                    }

                                    Propagation::Stop
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
                                tx.send_spawn(UIEvent::Remove(id));

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
