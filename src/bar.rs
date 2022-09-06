use crate::broadcaster::BridgeChannel;
use crate::config::{BarPosition, ModuleConfig};
use crate::modules::{Module, ModuleInfoBuilder, ModuleLocation, ModuleUpdateEvent, WidgetContext};
use crate::popup::Popup;
use crate::Config;
use chrono::{DateTime, Local};
use color_eyre::Result;
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Orientation};
use std::sync::{Arc, RwLock};
use tokio::spawn;
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Creates a new window for a bar,
/// sets it up and adds its widgets.
pub fn create_bar(
    app: &Application,
    monitor: &Monitor,
    monitor_name: &str,
    config: Config,
) -> Result<()> {
    let win = ApplicationWindow::builder().application(app).build();

    setup_layer_shell(&win, monitor, &config.position);

    let content = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(0)
        .hexpand(false)
        .height_request(config.height)
        .name("bar")
        .build();

    let left = gtk::Box::builder().spacing(0).name("left").build();
    let center = gtk::Box::builder().spacing(0).name("center").build();
    let right = gtk::Box::builder().spacing(0).name("right").build();

    content.style_context().add_class("container");
    left.style_context().add_class("container");
    center.style_context().add_class("container");
    right.style_context().add_class("container");

    content.add(&left);
    content.set_center_widget(Some(&center));
    content.pack_end(&right, false, false, 0);

    load_modules(&left, &center, &right, app, config, monitor, monitor_name)?;
    win.add(&content);

    win.connect_destroy_event(|_, _| {
        info!("Shutting down");
        gtk::main_quit();
        Inhibit(false)
    });

    debug!("Showing bar");
    win.show_all();

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
        .bar_position(&config.position)
        .monitor(monitor)
        .output_name(output_name);

    if let Some(modules) = config.left {
        let mut info_builder = info_builder.location(ModuleLocation::Left);

        add_modules(left, modules, &mut info_builder)?;
    }

    if let Some(modules) = config.center {
        let mut info_builder = info_builder.location(ModuleLocation::Left);

        add_modules(center, modules, &mut info_builder)?;
    }

    if let Some(modules) = config.right {
        let mut info_builder = info_builder.location(ModuleLocation::Left);

        add_modules(right, modules, &mut info_builder)?;
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
    macro_rules! add_module {
        ($module:expr, $name:literal) => {{
            // let widget = $module.into_widget(&info)?;
            // widget.set_widget_name($name);
            // content.add(&widget);
            debug!("Added module of type {}", $name);
        }};
    }

    let base_popup_info = info_builder.module_name("").build()?;
    let popup = Popup::new(base_popup_info);
    let popup = Arc::new(RwLock::new(popup));

    for config in modules {
        match config {
            ModuleConfig::Clock(module) => {
                let info = info_builder.module_name("clock").build()?;
                // let broadcaster = GLibBroadcaster::<DateTime<Local>>::new();
                // setup_module(Box::new(module), content, info);
                let (w_tx, w_rx) = glib::MainContext::channel::<DateTime<Local>>(glib::PRIORITY_DEFAULT);
                let (p_tx, p_rx) = glib::MainContext::channel::<DateTime<Local>>(glib::PRIORITY_DEFAULT);

                // let (tx, mut rx) = mpsc::channel::<ModuleUpdateEvent<DateTime<Local>>>(32);
                let channel = BridgeChannel::<ModuleUpdateEvent<DateTime<Local>>>::new();

                module.spawn_controller(&info, tx.clone())?;

                let context = WidgetContext { id: String::from("clock"), widget_rx: w_rx, popup_rx: p_rx, tx };

                let widget = module.into_widget(context)?;
                
                let popup2 = Arc::clone(&popup);
                spawn(async move {
                    let popup = popup2;
                    while let Some(ev) = rx.recv().await {
                        match ev {
                            ModuleUpdateEvent::Update(update) => {
                                p_tx.send(update)?;
                                w_tx.send(update)?;
                            },
                            ModuleUpdateEvent::TogglePopup => {
                                popup.read().expect("Failed to get read lock on popup").show_content("clock");
                            }
                        }
                    }

                    Ok::<(), std::sync::mpsc::SendError<DateTime<Local>>>(())
                });

                content.add(&widget.widget);
                widget.widget.set_widget_name(info.module_name);

                if let Some(popup_content) = widget.popup {
                    popup.write().expect("Failed to get write lock on popup").register_content(String::from("clock"), popup_content);
                }

            }
             // ModuleConfig::Clock(module) => add_module!(module, "clock"),
             // ModuleConfig::Mpd(module) => add_module!(module, "mpd"),
             // ModuleConfig::Tray(module) => add_module!(module, "tray"),
             // ModuleConfig::Workspaces(module) => add_module!(module, "workspaces"),
             // ModuleConfig::SysInfo(module) => add_module!(module, "sysinfo"),
             // ModuleConfig::Launcher(module) => add_module!(module, "launcher"),
             // ModuleConfig::Script(module) => add_module!(module, "script"),
             // ModuleConfig::Focused(module) => add_module!(module, "focused"),
        }
    }

    Ok(())
}

/// Sets up GTK layer shell for a provided aplication window.
fn setup_layer_shell(win: &ApplicationWindow, monitor: &Monitor, position: &BarPosition) {
    gtk_layer_shell::init_for_window(win);
    gtk_layer_shell::set_monitor(win, monitor);
    gtk_layer_shell::set_layer(win, gtk_layer_shell::Layer::Top);
    gtk_layer_shell::auto_exclusive_zone_enable(win);

    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Top, 0);
    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Bottom, 0);
    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Left, 0);
    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Right, 0);

    gtk_layer_shell::set_anchor(
        win,
        gtk_layer_shell::Edge::Top,
        position == &BarPosition::Top,
    );
    gtk_layer_shell::set_anchor(
        win,
        gtk_layer_shell::Edge::Bottom,
        position == &BarPosition::Bottom,
    );
    gtk_layer_shell::set_anchor(win, gtk_layer_shell::Edge::Left, true);
    gtk_layer_shell::set_anchor(win, gtk_layer_shell::Edge::Right, true);
}
