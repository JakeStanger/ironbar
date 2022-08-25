use crate::collection::Collection;
use crate::icon::{find_desktop_file, get_icon};
use crate::modules::launcher::popup::Popup;
use crate::modules::launcher::FocusEvent;
use crate::sway::SwayNode;
use crate::Report;
use color_eyre::Help;
use gtk::prelude::*;
use gtk::{Button, IconTheme, Image};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use tokio::spawn;
use tokio::sync::mpsc;
use tracing::error;

#[derive(Debug, Clone)]
pub struct LauncherItem {
    pub app_id: String,
    pub favorite: bool,
    pub windows: Rc<Mutex<Collection<i32, LauncherWindow>>>,
    pub state: Arc<RwLock<State>>,
    pub button: Button,
}

#[derive(Debug, Clone)]
pub struct LauncherWindow {
    pub con_id: i32,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OpenState {
    Closed,
    Open,
    Focused,
    Urgent,
}

impl OpenState {
    pub const fn from_node(node: &SwayNode) -> Self {
        if node.focused {
            Self::Urgent
        } else if node.urgent {
            Self::Focused
        } else {
            Self::Open
        }
    }

    pub fn highest_of(a: &Self, b: &Self) -> Self {
        if a == &Self::Urgent || b == &Self::Urgent {
            Self::Urgent
        } else if a == &Self::Focused || b == &Self::Focused {
            Self::Focused
        } else if a == &Self::Open || b == &Self::Open {
            Self::Open
        } else {
            Self::Closed
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub is_xwayland: bool,
    pub open_state: OpenState,
}

#[derive(Debug, Clone)]
pub struct ButtonConfig {
    pub icon_theme: IconTheme,
    pub show_names: bool,
    pub show_icons: bool,
    pub popup: Popup,
    pub tx: mpsc::Sender<FocusEvent>,
}

impl LauncherItem {
    pub fn new(app_id: String, favorite: bool, config: &ButtonConfig) -> Self {
        let button = Button::new();
        button.style_context().add_class("item");

        let state = State {
            open_state: OpenState::Closed,
            is_xwayland: false,
        };

        let item = Self {
            app_id,
            favorite,
            windows: Rc::new(Mutex::new(Collection::new())),
            state: Arc::new(RwLock::new(state)),
            button,
        };

        item.configure_button(config);
        item
    }

    pub fn from_node(node: &SwayNode, config: &ButtonConfig) -> Self {
        let button = Button::new();
        button.style_context().add_class("item");

        let windows = Collection::from((
            node.id,
            LauncherWindow {
                con_id: node.id,
                name: node.name.clone(),
            },
        ));

        let state = State {
            open_state: OpenState::from_node(node),
            is_xwayland: node.is_xwayland(),
        };

        let item = Self {
            app_id: node.get_id().to_string(),
            favorite: false,
            windows: Rc::new(Mutex::new(windows)),
            state: Arc::new(RwLock::new(state)),
            button,
        };

        item.configure_button(config);
        item
    }

    fn configure_button(&self, config: &ButtonConfig) {
        let button = &self.button;

        let windows = self.windows.lock().expect("Failed to get lock on windows");

        let name = if windows.len() == 1 {
            windows
                .first()
                .expect("Failed to get first window")
                .name
                .as_ref()
        } else {
            Some(&self.app_id)
        };

        if let Some(name) = name {
            self.set_title(name, config);
        }

        if config.show_icons {
            let icon = get_icon(&config.icon_theme, &self.app_id, 32);
            if icon.is_some() {
                let image = Image::from_pixbuf(icon.as_ref());
                button.set_image(Some(&image));
                button.set_always_show_image(true);
            }
        }

        let app_id = self.app_id.clone();
        let state = Arc::clone(&self.state);
        let tx_click = config.tx.clone();

        let (focus_tx, mut focus_rx) = mpsc::channel(32);

        button.connect_clicked(move |_| {
            let state = state.read().expect("Failed to get read lock on state");
            if state.open_state != OpenState::Closed {
                focus_tx.try_send(()).expect("Failed to send focus event");
            } else {
                // attempt to find desktop file and launch
                match find_desktop_file(&app_id) {
                    Some(file) => {
                        if let Err(err) = Command::new("gtk-launch")
                            .arg(
                                file.file_name()
                                    .expect("File segment missing from path to desktop file"),
                            )
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                        {
                            error!(
                                "{:?}",
                                Report::new(err)
                                    .wrap_err("Failed to run gtk-launch command.")
                                    .suggestion("Perhaps the desktop file is invalid?")
                            );
                        }
                    }
                    None => error!("Could not find desktop file for {}", app_id),
                }
            }
        });

        let app_id = self.app_id.clone();
        let state = Arc::clone(&self.state);

        spawn(async move {
            while focus_rx.recv().await == Some(()) {
                let state = state.read().expect("Failed to get read lock on state");
                if state.is_xwayland {
                    tx_click
                        .try_send(FocusEvent::Class(app_id.clone()))
                        .expect("Failed to send focus event");
                } else {
                    tx_click
                        .try_send(FocusEvent::AppId(app_id.clone()))
                        .expect("Failed to send focus event");
                }
            }
        });

        let popup = config.popup.clone();
        let popup2 = config.popup.clone();
        let windows = Rc::clone(&self.windows);
        let tx_hover = config.tx.clone();

        button.connect_enter_notify_event(move |button, _| {
            let windows = windows.lock().expect("Failed to get lock on windows");
            if windows.len() > 1 {
                popup.set_windows(windows.as_slice(), &tx_hover);
                popup.show(button);
            }

            Inhibit(false)
        });

        {}

        button.connect_leave_notify_event(move |_, e| {
            let (_, y) = e.position();
            // hover boundary
            if y > 2.0 {
                popup2.hide();
            }

            Inhibit(false)
        });

        let style = button.style_context();

        style.add_class("launcher-item");
        self.update_button_classes(&self.state.read().expect("Failed to get read lock on state"));

        button.show_all();
    }

    pub fn set_title(&self, title: &str, config: &ButtonConfig) {
        if config.show_names {
            self.button.set_label(title);
        } else {
            self.button.set_tooltip_text(Some(title));
        };
    }

    /// Updates the classnames on the GTK button
    /// based on its current state.
    ///
    /// State must be passed as an arg here rather than
    /// using `self.state` to avoid a weird `RwLock` issue.
    pub fn update_button_classes(&self, state: &State) {
        let style = self.button.style_context();

        if self.favorite {
            style.add_class("favorite");
        } else {
            style.remove_class("favorite");
        }

        if state.open_state == OpenState::Open {
            style.add_class("open");
        } else {
            style.remove_class("open");
        }

        if state.open_state == OpenState::Focused {
            style.add_class("focused");
        } else {
            style.remove_class("focused");
        }

        if state.open_state == OpenState::Urgent {
            style.add_class("urgent");
        } else {
            style.remove_class("urgent");
        }
    }
}
