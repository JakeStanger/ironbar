use crate::icon;
use crate::modules::{Module, ModuleInfo};
use crate::sway::node::get_open_windows;
use crate::sway::WindowEvent;
use glib::Continue;
use gtk::prelude::*;
use gtk::{IconTheme, Image, Label, Orientation};
use ksway::{Client, IpcEvent};
use serde::Deserialize;
use tokio::task::spawn_blocking;

#[derive(Debug, Deserialize, Clone)]
pub struct FocusedModule {
    #[serde(default = "crate::config::default_true")]
    show_icon: bool,
    #[serde(default = "crate::config::default_true")]
    show_title: bool,

    #[serde(default = "default_icon_size")]
    icon_size: i32,
    icon_theme: Option<String>,
}

const fn default_icon_size() -> i32 {
    32
}

impl Module<gtk::Box> for FocusedModule {
    fn into_widget(self, _info: &ModuleInfo) -> gtk::Box {
        let icon_theme = IconTheme::new();

        if let Some(theme) = self.icon_theme {
            icon_theme.set_custom_theme(Some(&theme));
        }

        let container = gtk::Box::new(Orientation::Horizontal, 5);

        let icon = Image::builder().name("icon").build();
        let label = Label::builder().name("label").build();

        container.add(&icon);
        container.add(&label);

        let mut sway = Client::connect().unwrap();

        let srx = sway.subscribe(vec![IpcEvent::Window]).unwrap();
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let focused = get_open_windows(&mut sway)
            .into_iter()
            .find(|node| node.focused);

        if let Some(focused) = focused {
            tx.send(focused).unwrap();
        }

        spawn_blocking(move || loop {
            while let Ok((_, payload)) = srx.try_recv() {
                let payload: WindowEvent = serde_json::from_slice(&payload).unwrap();

                let update = match payload.change.as_str() {
                    "focus" => true,
                    "title" => payload.container.focused,
                    _ => false,
                };

                if update {
                    tx.send(payload.container).unwrap();
                }
            }
            sway.poll().unwrap();
        });

        {
            rx.attach(None, move |node| {
                let value = node
                    .name
                    .as_deref()
                    .unwrap_or_else(|| node.get_id());

                let pixbuf = icon::get_icon(&icon_theme, node.get_id(), self.icon_size);

                if self.show_icon {
                    icon.set_pixbuf(pixbuf.as_ref());
                }

                if self.show_title {
                    label.set_label(value);
                }

                Continue(true)
            });
        }

        container
    }
}
