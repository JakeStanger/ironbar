use crate::bridge_channel::BridgeChannel;
use crate::config::{BarPosition, ModuleConfig};
use crate::modules::custom::ExecEvent;
use crate::modules::launcher::{ItemEvent, LauncherUpdate};
use crate::modules::mpd::{PlayerCommand, SongUpdate};
use crate::modules::workspaces::WorkspaceUpdate;
use crate::modules::{Module, ModuleInfoBuilder, ModuleLocation, ModuleUpdateEvent, WidgetContext};
use crate::popup::Popup;
use crate::script::{OutputStream, Script};
use crate::widgets::DynamicString;
use crate::{await_sync, Config};
use chrono::{DateTime, Local};
use color_eyre::Result;
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Orientation};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use stray::message::NotifierItemCommand;
use stray::NotifierItemMessage;
use tokio::spawn;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace};

/// Creates a new window for a bar,
/// sets it up and adds its widgets.
pub fn create_bar(
    app: &Application,
    monitor: &Monitor,
    monitor_name: &str,
    config: Config,
) -> Result<()> {
    let win = ApplicationWindow::builder().application(app).build();

    setup_layer_shell(&win, monitor, config.position, config.anchor_to_edges);

    let orientation = config.position.get_orientation();

    let content = gtk::Box::builder()
        .orientation(orientation)
        .spacing(0)
        .hexpand(false)
        .name("bar");

    let content = if orientation == Orientation::Horizontal {
        content.height_request(config.height)
    } else {
        content.width_request(config.height)
    }
    .build();

    let start = gtk::Box::builder()
        .orientation(orientation)
        .spacing(0)
        .name("start")
        .build();
    let center = gtk::Box::builder()
        .orientation(orientation)
        .spacing(0)
        .name("center")
        .build();
    let end = gtk::Box::builder()
        .orientation(orientation)
        .spacing(0)
        .name("end")
        .build();

    content.style_context().add_class("container");
    start.style_context().add_class("container");
    center.style_context().add_class("container");
    end.style_context().add_class("container");

    content.add(&start);
    content.set_center_widget(Some(&center));
    content.pack_end(&end, false, false, 0);

    load_modules(&start, &center, &end, app, config, monitor, monitor_name)?;
    win.add(&content);

    win.connect_destroy_event(|_, _| {
        info!("Shutting down");
        gtk::main_quit();
        Inhibit(false)
    });

    debug!("Showing bar");
    start.show();
    center.show();
    end.show();
    content.show();
    win.show();

    Ok(())
}

/// Loads the configured modules onto a bar.
fn load_modules(
    left: &gtk::Box,
    center: &gtk::Box,
    right: &gtk::Box,
    app: &Application,
    config: Config,
    monitor: &Monitor,
    output_name: &str,
) -> Result<()> {
    let mut info_builder = ModuleInfoBuilder::default();
    let info_builder = info_builder
        .app(app)
        .bar_position(config.position)
        .monitor(monitor)
        .output_name(output_name);

    if let Some(modules) = config.start {
        let info_builder = info_builder.location(ModuleLocation::Left);

        add_modules(left, modules, info_builder)?;
    }

    if let Some(modules) = config.center {
        let info_builder = info_builder.location(ModuleLocation::Center);

        add_modules(center, modules, info_builder)?;
    }

    if let Some(modules) = config.end {
        let info_builder = info_builder.location(ModuleLocation::Right);

        add_modules(right, modules, info_builder)?;
    }

    Ok(())
}

/// Adds modules into a provided GTK box,
/// which should be one of its left, center or right containers.
fn add_modules(
    content: &gtk::Box,
    modules: Vec<ModuleConfig>,
    info_builder: &mut ModuleInfoBuilder,
) -> Result<()> {
    let base_popup_info = info_builder.module_name("").build()?;
    let popup = Popup::new(&base_popup_info);
    let popup = Arc::new(RwLock::new(popup));

    macro_rules! add_module {
        ($module:expr, $id:expr, $name:literal, $send_message:ty, $receive_message:ty) => {
            let info = info_builder.module_name($name).build()?;

            let (w_tx, w_rx) = glib::MainContext::channel::<$send_message>(glib::PRIORITY_DEFAULT);
            let (p_tx, p_rx) = glib::MainContext::channel::<$send_message>(glib::PRIORITY_DEFAULT);

            let channel = BridgeChannel::<ModuleUpdateEvent<$send_message>>::new();
            let (ui_tx, ui_rx) = mpsc::channel::<$receive_message>(16);

            $module.spawn_controller(&info, channel.create_sender(), ui_rx)?;

            let context = WidgetContext {
                id: $id,
                widget_rx: w_rx,
                popup_rx: p_rx,
                tx: channel.create_sender(),
                controller_tx: ui_tx,
            };

            let common = $module.common.clone();

            let widget = $module.into_widget(context, &info)?;

            let container = gtk::EventBox::new();
            container.add(&widget.widget);

            content.add(&container);
            widget.widget.set_widget_name(info.module_name);

            if let Some(show_if) = common.show_if {
                let script = Script::new_polling(show_if);
                let container = container.clone();

                let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

                spawn(async move {
                    script
                        .run(|(_, success)| {
                            tx.send(success)
                                .expect("Failed to send widget visibility toggle message");
                        })
                        .await;
                });

                rx.attach(None, move |success| {
                    if success {
                        container.show_all()
                    } else {
                        container.hide()
                    };
                    Continue(true)
                });
            } else {
                container.show_all();
            }

            if let Some(on_click) = common.on_click {
                let script = Script::new_polling(on_click);
                container.connect_button_press_event(move |_, _| {
                    trace!("Running on-click script");
                    match await_sync(async { script.get_output().await }) {
                        Ok((OutputStream::Stderr(out), _)) => error!("{out}"),
                        Err(err) => error!("{err:?}"),
                        _ => {}
                    }
                    Inhibit(false)
                });
            }

            if let Some(tooltip) = common.tooltip {
                DynamicString::new(&tooltip, move |string| {
                    container.set_tooltip_text(Some(&string));
                    Continue(true)
                });
            }

            let has_popup = widget.popup.is_some();
            if let Some(popup_content) = widget.popup {
                popup
                    .write()
                    .expect("Failed to get write lock on popup")
                    .register_content($id, popup_content);
            }

            let popup2 = Arc::clone(&popup);
            channel.recv(move |ev| {
                let popup = popup2.clone();
                match ev {
                    ModuleUpdateEvent::Update(update) => {
                        if has_popup {
                            p_tx.send(update.clone())
                                .expect("Failed to send update to popup");
                        }

                        w_tx.send(update).expect("Failed to send update to module");
                    }
                    ModuleUpdateEvent::TogglePopup(geometry) => {
                        debug!("Toggling popup for {} [#{}]", $name, $id);
                        let popup = popup.read().expect("Failed to get read lock on popup");
                        if popup.is_visible() {
                            popup.hide()
                        } else {
                            popup.show_content($id);
                            popup.show(geometry);
                        }
                    }
                    ModuleUpdateEvent::OpenPopup(geometry) => {
                        debug!("Opening popup for {} [#{}]", $name, $id);

                        let popup = popup.read().expect("Failed to get read lock on popup");
                        popup.hide();
                        popup.show_content($id);
                        popup.show(geometry);
                    }
                    ModuleUpdateEvent::ClosePopup => {
                        debug!("Closing popup for {} [#{}]", $name, $id);

                        let popup = popup.read().expect("Failed to get read lock on popup");
                        popup.hide();
                    }
                }

                Continue(true)
            });
        };
    }

    for (id, config) in modules.into_iter().enumerate() {
        match config {
            ModuleConfig::Clock(module) => {
                add_module!(module, id, "clock", DateTime<Local>, ());
            }
            ModuleConfig::Script(module) => {
                add_module!(module, id, "script", String, ());
            }
            ModuleConfig::SysInfo(module) => {
                add_module!(module, id, "sysinfo", HashMap<String, String>, ());
            }
            ModuleConfig::Focused(module) => {
                add_module!(module, id, "focused", (String, String), ());
            }
            ModuleConfig::Workspaces(module) => {
                add_module!(module, id, "workspaces", WorkspaceUpdate, String);
            }
            ModuleConfig::Tray(module) => {
                add_module!(module, id, "tray", NotifierItemMessage, NotifierItemCommand);
            }
            ModuleConfig::Mpd(module) => {
                add_module!(module, id, "mpd", Option<SongUpdate>, PlayerCommand);
            }
            ModuleConfig::Launcher(module) => {
                add_module!(module, id, "launcher", LauncherUpdate, ItemEvent);
            }
            ModuleConfig::Custom(module) => {
                add_module!(module, id, "custom", (), ExecEvent);
            }
        }
    }

    Ok(())
}

/// Sets up GTK layer shell for a provided application window.
fn setup_layer_shell(
    win: &ApplicationWindow,
    monitor: &Monitor,
    position: BarPosition,
    anchor_to_edges: bool,
) {
    gtk_layer_shell::init_for_window(win);
    gtk_layer_shell::set_monitor(win, monitor);
    gtk_layer_shell::set_layer(win, gtk_layer_shell::Layer::Top);
    gtk_layer_shell::auto_exclusive_zone_enable(win);

    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Top, 0);
    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Bottom, 0);
    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Left, 0);
    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Right, 0);

    let bar_orientation = position.get_orientation();

    gtk_layer_shell::set_anchor(
        win,
        gtk_layer_shell::Edge::Top,
        position == BarPosition::Top
            || (bar_orientation == Orientation::Vertical && anchor_to_edges),
    );
    gtk_layer_shell::set_anchor(
        win,
        gtk_layer_shell::Edge::Bottom,
        position == BarPosition::Bottom
            || (bar_orientation == Orientation::Vertical && anchor_to_edges),
    );
    gtk_layer_shell::set_anchor(
        win,
        gtk_layer_shell::Edge::Left,
        position == BarPosition::Left
            || (bar_orientation == Orientation::Horizontal && anchor_to_edges),
    );
    gtk_layer_shell::set_anchor(
        win,
        gtk_layer_shell::Edge::Right,
        position == BarPosition::Right
            || (bar_orientation == Orientation::Horizontal && anchor_to_edges),
    );
}
