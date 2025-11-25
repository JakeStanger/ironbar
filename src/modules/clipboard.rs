use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::clipboard::{self, ClipboardEvent};
use crate::clients::wayland::{ClipboardItem, ClipboardValue};
use crate::config::{CommonConfig, LayoutConfig, TruncateMode};
use crate::gtk_helpers::IronbarLabelExt;
use crate::gtk_helpers::IronbarPaintableExt;
use crate::image::IconButton;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{module_impl, spawn};
use gtk::gdk::{BUTTON_PRIMARY, Texture};
use gtk::prelude::*;
use gtk::{Button, CheckButton, ContentFit, GestureClick, Label, Orientation, Picture, Widget};
use serde::Deserialize;
use std::collections::HashMap;
use std::ops::Deref;
use tokio::sync::mpsc;
use tracing::{debug, error};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ClipboardModule {
    /// The icon to show on the bar widget button.
    /// Supports [image](images) icons.
    ///
    /// **Default**: `󰨸`
    icon: String,

    /// The size to render the icon at.
    /// Note this only applies to image-type icons.
    ///
    /// **Default**: `32`
    icon_size: i32,

    /// The maximum number of items to keep in the history,
    /// and to show in the popup.
    ///
    /// **Default**: `10`
    max_items: usize,

    /// The maximum width to render copied images at.
    ///
    /// **Default**: `256.0`
    image_max_width: f64,

    /// The maximum height to render copied images at.
    ///
    /// **Default**: `64.0`
    image_max_height: f64,

    // -- Common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    truncate: Option<TruncateMode>,

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for ClipboardModule {
    fn default() -> Self {
        Self {
            icon: "󰨸".to_string(),
            icon_size: 32,
            max_items: 10,
            image_max_width: 256.0,
            image_max_height: 64.0,
            truncate: None,
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
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
    ) -> miette::Result<()> {
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
    ) -> miette::Result<ModuleParts<Button>> {
        let button = IconButton::new(&self.icon, self.icon_size, context.ironbar.image_provider());

        button.label().set_justify(self.layout.justify.into());
        button.add_css_class("btn");

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        let popup = self
            .into_popup(context, info)
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button.deref().clone(), popup))
    }

    fn into_popup(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        let container = gtk::Box::new(Orientation::Vertical, 10);

        let entries = gtk::Box::new(Orientation::Vertical, 5);
        container.append(&entries);

        let hidden_option = CheckButton::new();
        entries.append(&hidden_option);

        let mut items = HashMap::new();

        context
            .subscribe()
            .recv_glib(&hidden_option, move |hidden_option, event| {
                match event {
                    ControllerEvent::Add(id, item) => {
                        debug!("Adding new value with ID {}", id);

                        let row = gtk::Box::new(Orientation::Horizontal, 0);
                        row.add_css_class("item");

                        let button = match item.value.as_ref() {
                            ClipboardValue::Text(value) => {
                                let button = CheckButton::builder().group(hidden_option).build();

                                let label = Label::new(Some(value));
                                label.set_xalign(0.1);
                                button.set_child(Some(&label));

                                if let Some(truncate) = self.truncate {
                                    label.truncate(truncate);
                                }

                                button.add_css_class("text");
                                button
                            }
                            ClipboardValue::Image(bytes) => match Texture::from_bytes(bytes) {
                                Ok(texture) => {
                                    let texture =
                                        texture.scale(self.image_max_width, self.image_max_height);

                                    let image = Picture::new();
                                    image.set_content_fit(ContentFit::ScaleDown);
                                    image.set_paintable(texture.as_ref());

                                    let button =
                                        CheckButton::builder().group(hidden_option).build();
                                    button.set_child(Some(&image));
                                    button.add_css_class("image");

                                    button
                                }
                                Err(err) => {
                                    error!("{err:?}");
                                    return;
                                }
                            },
                            ClipboardValue::Other => unreachable!(),
                        };

                        button.add_css_class("btn");
                        button.set_active(true); // if just added, should be on clipboard

                        button.set_widget_name(&format!("copy-{id}"));

                        {
                            let tx = context.controller_tx.clone();
                            let button2 = button.clone();

                            let event_handler =
                                GestureClick::builder().button(BUTTON_PRIMARY).build();

                            event_handler.connect_pressed(move |_, _, _, _| {
                                let id = get_button_id(&button2)
                                    .expect("Failed to get id from button name");

                                debug!("Copying item with id: {id}");
                                tx.send_spawn(UIEvent::Copy(id));
                            });

                            button.add_controller(event_handler);
                        }

                        let remove_button = Button::with_label("x");
                        remove_button.set_widget_name(&format!("remove-{id}"));
                        remove_button.add_css_class("btn-remove");

                        {
                            let tx = context.controller_tx.clone();
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

                        row.prepend(&button);
                        row.append(&remove_button);
                        button.set_hexpand(true);

                        entries.prepend(&row);

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

        hidden_option.set_visible(false);

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
