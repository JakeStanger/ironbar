use crate::collection::Collection;
use crate::icon::{find_desktop_file, get_icon};
use crate::modules::launcher::popup::Popup;
use crate::modules::launcher::FocusEvent;
use crate::popup::PopupAlignment;
use crate::sway::SwayNode;
use gtk::prelude::*;
use gtk::{Button, IconTheme, Image};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use tokio::spawn;
use tokio::sync::mpsc;

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

#[derive(Debug, Clone)]
pub struct State {
    pub is_xwayland: bool,
    pub open: bool,
    pub focused: bool,
    pub urgent: bool,
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
            open: false,
            focused: false,
            urgent: false,
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
            open: true,
            focused: node.focused,
            urgent: node.urgent,
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

        let windows = self.windows.lock().unwrap();

        let name = if windows.len() == 1 {
            windows.first().unwrap().name.as_ref()
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
            let state = state.read().unwrap();
            if state.open {
                focus_tx.try_send(()).unwrap();
            } else {
                // attempt to find desktop file and launch
                match find_desktop_file(&app_id) {
                    Some(file) => {
                        Command::new("gtk-launch")
                            .arg(file.file_name().unwrap())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .unwrap();
                    }
                    None => (),
                }
            }
        });

        let app_id = self.app_id.clone();
        let state = Arc::clone(&self.state);

        spawn(async move {
            while focus_rx.recv().await == Some(()) {
                let state = state.read().unwrap();
                if state.is_xwayland {
                    tx_click
                        .try_send(FocusEvent::Class(app_id.clone()))
                        .unwrap();
                } else {
                    tx_click
                        .try_send(FocusEvent::AppId(app_id.clone()))
                        .unwrap();
                }
            }
        });

        let popup = config.popup.clone();
        let popup2 = config.popup.clone();
        let windows = Rc::clone(&self.windows);
        let tx_hover = config.tx.clone();

        button.connect_enter_notify_event(move |button, _| {
            let windows = windows.lock().unwrap();
            if windows.len() > 1 {
                let button_w = button.allocation().width();

                let (button_x, _) = button
                    .translate_coordinates(&button.toplevel().unwrap(), 0, 0)
                    .unwrap();

                let button_center = f64::from(button_x) + f64::from(button_w) / 2.0;

                popup.set_windows(windows.as_slice(), &tx_hover);
                popup.show();

                // TODO: Pass through module location
                popup.set_pos(button_center, PopupAlignment::Center);
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
        self.update_button_classes(&self.state.read().unwrap());

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

        if state.open {
            style.add_class("open");
        } else {
            style.remove_class("open");
        }

        if state.focused {
            style.add_class("focused");
        } else {
            style.remove_class("focused");
        }

        if state.urgent {
            style.add_class("urgent");
        } else {
            style.remove_class("urgent");
        }
    }
}
