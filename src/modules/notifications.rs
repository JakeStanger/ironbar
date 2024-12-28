use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::swaync;
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};
use gtk::prelude::*;
use gtk::{Align, Button, Label, Overlay};
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;
use tracing::error;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NotificationsModule {
    /// Whether to show the current notification count.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    show_count: bool,

    /// SwayNC state icons.
    ///
    /// See [icons](#icons).
    #[serde(default)]
    icons: Icons,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct Icons {
    /// Icon to show when the panel is closed, with no notifications.
    ///
    /// **Default**: `󰍥`
    #[serde(default = "default_icon_closed_none")]
    closed_none: String,

    /// Icon to show when the panel is closed, with notifications.
    ///
    /// **Default**: `󱥂`
    #[serde(default = "default_icon_closed_some")]
    closed_some: String,

    /// Icon to show when the panel is closed, with DnD enabled.
    /// Takes higher priority than count-based icons.
    ///
    /// **Default**: `󱅯`
    #[serde(default = "default_icon_closed_dnd")]
    closed_dnd: String,

    /// Icon to show when the panel is open, with no notifications.
    ///
    /// **Default**: `󰍡`
    #[serde(default = "default_icon_open_none")]
    open_none: String,

    /// Icon to show when the panel is open, with notifications.
    ///
    /// **Default**: `󱥁`
    #[serde(default = "default_icon_open_some")]
    open_some: String,

    /// Icon to show when the panel is open, with DnD enabled.
    /// Takes higher priority than count-based icons.
    ///
    /// **Default**: `󱅮`
    #[serde(default = "default_icon_open_dnd")]
    open_dnd: String,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            closed_none: default_icon_closed_none(),
            closed_some: default_icon_closed_some(),
            closed_dnd: default_icon_closed_dnd(),
            open_none: default_icon_open_none(),
            open_some: default_icon_open_some(),
            open_dnd: default_icon_open_dnd(),
        }
    }
}

fn default_icon_closed_none() -> String {
    String::from("󰍥")
}

fn default_icon_closed_some() -> String {
    String::from("󱥂")
}

fn default_icon_closed_dnd() -> String {
    String::from("󱅯")
}

fn default_icon_open_none() -> String {
    String::from("󰍡")
}

fn default_icon_open_some() -> String {
    String::from("󱥁")
}

fn default_icon_open_dnd() -> String {
    String::from("󱅮")
}

impl Icons {
    fn icon(&self, value: swaync::Event) -> &str {
        match (value.cc_open, value.count > 0, value.dnd) {
            (true, _, true) => &self.open_dnd,
            (true, true, false) => &self.open_some,
            (true, false, false) => &self.open_none,
            (false, _, true) => &self.closed_dnd,
            (false, true, false) => &self.closed_some,
            (false, false, false) => &self.closed_none,
        }
        .as_str()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UiEvent {
    ToggleVisibility,
}

impl Module<Overlay> for NotificationsModule {
    type SendMessage = swaync::Event;
    type ReceiveMessage = UiEvent;

    module_impl!("notifications");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> color_eyre::Result<()>
    where
        <Self as Module<Overlay>>::SendMessage: Clone,
    {
        let client = context.try_client::<swaync::Client>()?;

        {
            let client = client.clone();
            let mut rx = client.subscribe();
            let tx = context.tx.clone();

            spawn(async move {
                let initial_state = client.state().await;

                match initial_state {
                    Ok(ev) => tx.send_update(ev).await,
                    Err(err) => error!("{err:?}"),
                };

                while let Ok(ev) = rx.recv().await {
                    tx.send_update(ev).await;
                }
            });
        }

        spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    UiEvent::ToggleVisibility => client.toggle_visibility().await,
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> color_eyre::Result<ModuleParts<Overlay>>
    where
        <Self as Module<Overlay>>::SendMessage: Clone,
    {
        let overlay = Overlay::new();
        let button = Button::with_label(&self.icons.closed_none);
        overlay.add(&button);

        let label = Label::builder()
            .label("0")
            .halign(Align::End)
            .valign(Align::Start)
            .build();

        if self.show_count {
            label.add_class("count");
            overlay.add_overlay(&label);
            overlay.set_overlay_pass_through(&label, true);
        }

        let ctx = context.controller_tx.clone();
        button.connect_clicked(move |_| {
            ctx.send_spawn(UiEvent::ToggleVisibility);
        });

        {
            let button = button.clone();

            context.subscribe().recv_glib(move |ev| {
                let icon = self.icons.icon(ev);
                button.set_label(icon);

                label.set_label(&ev.count.to_string());
                label.set_visible(self.show_count && ev.count > 0);
            });
        }

        Ok(ModuleParts {
            widget: overlay,
            popup: None,
        })
    }
}
