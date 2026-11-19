use std::collections::HashMap;
use std::ops::Deref;

use color_eyre::Result;
use gtk::prelude::*;
use gtk::{Button, Orientation};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::notify::{self};
use crate::clients::udisks::{self, Drive, UdisksEvent};
use crate::config::{CommonConfig, LayoutConfig, TruncateMode};
use crate::image::IconButton;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{module_impl, spawn};
use tracing::{debug, error};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct UdisksModule {
    /// The icon to show on the bar widget button.
    /// Supports [image](images) icons.
    ///
    /// **Default**: `🖴`
    icon: String,

    /// The size to render the icon at, in pixels.
    ///
    /// **Default**: `32`
    icon_size: i32,

    // -- Common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    truncate: Option<TruncateMode>,

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

    /// The icon to show on the eject button button.
    /// Supports [image](images) icons.
    ///
    /// **Default**: `⏏`
    eject_button_icon: String,

    /// The size to render the icon at.
    /// Note this only applies to image-type icons.
    ///
    /// **Default**: `32`
    eject_button_icon_size: i32,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for UdisksModule {
    fn default() -> Self {
        Self {
            icon: "🖴".to_string(),
            icon_size: 32,
            truncate: None,
            layout: LayoutConfig::default(),
            eject_button_icon: "⏏".to_string(),
            eject_button_icon_size: 32,
            common: Some(CommonConfig::default()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum UIEvent {
    Eject(Drive),
    Open(Drive),
}

#[derive(Debug, Clone)]
pub enum ControllerEvent {
    Add(Drive),
    Remove(Drive),
}

fn drive_label(drive: &Drive) -> String {
    let model = if drive.model.is_empty() {
        "Unknown drive"
    } else {
        drive.model.as_str()
    };

    if drive.size == 0 {
        model.to_string()
    } else {
        format!("{model} ({})", format_size(drive.size))
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1000.0 && unit < UNITS.len() - 1 {
        size /= 1000.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

impl Module<Button> for UdisksModule {
    type SendMessage = ControllerEvent;
    type ReceiveMessage = UIEvent;

    module_impl!("udisks");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let client = context.try_client::<udisks::Client>()?;

        let tx = context.tx.clone();
        let drives = client.drives();

        for drive in drives {
            tx.send_update_spawn(ControllerEvent::Add(drive));
        }

        // listen to udisks events
        spawn(async move {
            let mut rx = client.subscribe();

            while let Ok(event) = rx.recv().await {
                match event {
                    UdisksEvent::Added(drive) => {
                        tx.send_update_spawn(ControllerEvent::Add(drive));
                    }
                    UdisksEvent::Removed(drive) => {
                        tx.send_update_spawn(ControllerEvent::Remove(drive));
                    }
                }
            }

            error!("Udisks client unexpectedly closed");
        });

        let client = context.try_client::<udisks::Client>()?;
        let notify_client = context.try_client::<notify::Client>()?;

        // listen to ui events
        spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    UIEvent::Eject(drive) => {
                        if client.eject(&drive.object_path).await.is_ok() {
                            let summary = format!(
                                "The '{}' device can now be safely reemoveed from the computer.",
                                drive.model
                            );
                            let _ = notify_client
                                .notify("Safe to remove hardware.", &summary, "media-eject")
                                .await;
                        } else {
                            let summary =
                                format!("Cant remove '{}', device may be busy.", drive.model);
                            let _ = notify_client
                                .notify("Error ejecting device.", &summary, "dialog-warning")
                                .await;
                        }
                    }
                    UIEvent::Open(drive) => match client
                        .open_folder(&drive.object_path)
                        .await
                        .map_err(|e| format!("{e:?}"))
                    {
                        Ok(_) => {
                            debug!("Opening {} containing folder", &drive.model);
                        }
                        Err(err) => {
                            let summary = format!("Cant open '{}' containing folder.", drive.model);

                            let _ = notify_client
                                .notify(
                                    "Error opening containing folder.",
                                    &summary,
                                    "dialog-error",
                                )
                                .await;
                            debug!("Can not open {} containing folder", &drive.model);
                            error!("{:?}", err);
                        }
                    },
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
        let button = IconButton::new(
            &self.icon,
            self.icon_size,
            &context.ironbar.image_provider(),
        );

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

        let eject_icon = self.eject_button_icon;
        let eject_icon_size = self.eject_button_icon_size;
        let image_provider = context.ironbar.image_provider();
        let controller_tx = context.controller_tx.clone();

        let mut items: HashMap<String, gtk::Box> = HashMap::new();

        context
            .subscribe()
            .recv_glib(&entries, move |entries, event| match event {
                ControllerEvent::Add(drive) => {
                    debug!("Adding drive");

                    let row = gtk::Box::new(Orientation::Horizontal, 10);
                    row.add_css_class("drive");

                    let open_button = Button::with_label(&drive_label(&drive));
                    open_button.add_css_class("drive-label");
                    open_button.add_css_class("flat");
                    open_button.set_hexpand(true);

                    let tx = controller_tx.clone();
                    let open_drive = drive.clone();
                    open_button.connect_clicked(move |_| {
                        tx.send_spawn(UIEvent::Open(open_drive.clone()));
                    });
                    row.append(&open_button);

                    let eject_button =
                        IconButton::new(&eject_icon, eject_icon_size, &image_provider);
                    eject_button.add_css_class("eject-button");

                    let tx = controller_tx.clone();
                    let eject_drive = drive.clone();
                    eject_button.connect_clicked(move |_| {
                        tx.send_spawn(UIEvent::Eject(eject_drive.clone()));
                    });
                    row.append(&*eject_button);

                    entries.append(&row);
                    items.insert(drive.id.clone(), row);
                }
                ControllerEvent::Remove(drive) => {
                    debug!("Removing drive");
                    if let Some(row) = items.remove(&drive.id) {
                        entries.remove(&row);
                    }
                }
            });

        Some(container)
    }
}
