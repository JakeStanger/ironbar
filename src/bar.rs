use crate::bridge_channel::BridgeChannel;
use crate::config::{BarPosition, CommonConfig, ModuleConfig};
use crate::dynamic_string::DynamicString;
use crate::modules::{Module, ModuleInfo, ModuleLocation, ModuleUpdateEvent, WidgetContext};
use crate::popup::Popup;
use crate::script::{OutputStream, Script};
use crate::{await_sync, read_lock, send, write_lock, Config};
use color_eyre::Result;
use gtk::gdk::{EventMask, Monitor, ScrollDirection};
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, EventBox, IconTheme, Orientation, Widget};
use std::sync::{Arc, RwLock};
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

    content.style_context().add_class("container");

    let start = create_container("start", orientation);
    let center = create_container("center", orientation);
    let end = create_container("end", orientation);

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

    // show each box but do not use `show_all`.
    // this ensures `show_if` option works as intended.
    start.show();
    center.show();
    end.show();
    content.show();
    win.show();

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
    gtk_layer_shell::set_namespace(win, env!("CARGO_PKG_NAME"));

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

/// Creates a `gtk::Box` container to place widgets inside.
fn create_container(name: &str, orientation: Orientation) -> gtk::Box {
    let container = gtk::Box::builder()
        .orientation(orientation)
        .spacing(0)
        .name(name)
        .build();

    container.style_context().add_class("container");
    container
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
    let icon_theme = IconTheme::new();
    if let Some(ref theme) = config.icon_theme {
        icon_theme.set_custom_theme(Some(theme));
    }

    macro_rules! info {
        ($location:expr) => {
            ModuleInfo {
                app,
                bar_position: config.position,
                monitor,
                output_name,
                location: $location,
                icon_theme: &icon_theme,
            }
        };
    }

    if let Some(modules) = config.start {
        let info = info!(ModuleLocation::Left);
        add_modules(left, modules, &info)?;
    }

    if let Some(modules) = config.center {
        let info = info!(ModuleLocation::Center);
        add_modules(center, modules, &info)?;
    }

    if let Some(modules) = config.end {
        let info = info!(ModuleLocation::Right);
        add_modules(right, modules, &info)?;
    }

    Ok(())
}

/// Adds modules into a provided GTK box,
/// which should be one of its left, center or right containers.
fn add_modules(content: &gtk::Box, modules: Vec<ModuleConfig>, info: &ModuleInfo) -> Result<()> {
    let popup = Popup::new(info);
    let popup = Arc::new(RwLock::new(popup));

    macro_rules! add_module {
        ($module:expr, $id:expr) => {{
            let common = $module.common.take().expect("Common config did not exist");
            let widget = create_module($module, $id, &info, &Arc::clone(&popup))?;

            let container = wrap_widget(&widget);
            content.add(&container);
            setup_module_common_options(container, common);
        }};
    }

    for (id, config) in modules.into_iter().enumerate() {
        match config {
            #[cfg(feature = "clock")]
            ModuleConfig::Clock(mut module) => add_module!(module, id),
            ModuleConfig::Custom(mut module) => add_module!(module, id),
            ModuleConfig::Focused(mut module) => add_module!(module, id),
            ModuleConfig::Launcher(mut module) => add_module!(module, id),
            #[cfg(feature = "music")]
            ModuleConfig::Music(mut module) => add_module!(module, id),
            ModuleConfig::Script(mut module) => add_module!(module, id),
            #[cfg(feature = "sys_info")]
            ModuleConfig::SysInfo(mut module) => add_module!(module, id),
            #[cfg(feature = "tray")]
            ModuleConfig::Tray(mut module) => add_module!(module, id),
            #[cfg(feature = "workspaces")]
            ModuleConfig::Workspaces(mut module) => add_module!(module, id),
        }
    }

    Ok(())
}

/// Creates a module and sets it up.
/// This setup includes widget/popup content and event channels.
fn create_module<TModule, TWidget, TSend, TRec>(
    module: TModule,
    id: usize,
    info: &ModuleInfo,
    popup: &Arc<RwLock<Popup>>,
) -> Result<TWidget>
where
    TModule: Module<TWidget, SendMessage = TSend, ReceiveMessage = TRec>,
    TWidget: IsA<Widget>,
    TSend: Clone + Send + 'static,
{
    let (w_tx, w_rx) = glib::MainContext::channel::<TSend>(glib::PRIORITY_DEFAULT);
    let (p_tx, p_rx) = glib::MainContext::channel::<TSend>(glib::PRIORITY_DEFAULT);

    let channel = BridgeChannel::<ModuleUpdateEvent<TSend>>::new();
    let (ui_tx, ui_rx) = mpsc::channel::<TRec>(16);

    module.spawn_controller(info, channel.create_sender(), ui_rx)?;

    let context = WidgetContext {
        id,
        widget_rx: w_rx,
        popup_rx: p_rx,
        tx: channel.create_sender(),
        controller_tx: ui_tx,
    };

    let name = TModule::name();

    let module_parts = module.into_widget(context, info)?;
    module_parts.widget.set_widget_name(name);

    let mut has_popup = false;
    if let Some(popup_content) = module_parts.popup {
        register_popup_content(popup, id, popup_content);
        has_popup = true;
    }

    setup_receiver(channel, w_tx, p_tx, popup.clone(), name, id, has_popup);

    Ok(module_parts.widget)
}

/// Registers the popup content with the popup.
fn register_popup_content(popup: &Arc<RwLock<Popup>>, id: usize, popup_content: gtk::Box) {
    write_lock!(popup).register_content(id, popup_content);
}

/// Sets up the bridge channel receiver
/// to pick up events from the controller, widget or popup.
///
/// Handles opening/closing popups
/// and communicating update messages between controllers and widgets/popups.
fn setup_receiver<TSend>(
    channel: BridgeChannel<ModuleUpdateEvent<TSend>>,
    w_tx: glib::Sender<TSend>,
    p_tx: glib::Sender<TSend>,
    popup: Arc<RwLock<Popup>>,
    name: &'static str,
    id: usize,
    has_popup: bool,
) where
    TSend: Clone + Send + 'static,
{
    channel.recv(move |ev| {
        match ev {
            ModuleUpdateEvent::Update(update) => {
                if has_popup {
                    send!(p_tx, update.clone());
                }

                send!(w_tx, update);
            }
            ModuleUpdateEvent::TogglePopup(geometry) => {
                debug!("Toggling popup for {} [#{}]", name, id);
                let popup = read_lock!(popup);
                if popup.is_visible() {
                    popup.hide();
                } else {
                    popup.show_content(id);
                    popup.show(geometry);
                }
            }
            ModuleUpdateEvent::OpenPopup(geometry) => {
                debug!("Opening popup for {} [#{}]", name, id);

                let popup = read_lock!(popup);
                popup.hide();
                popup.show_content(id);
                popup.show(geometry);
            }
            ModuleUpdateEvent::ClosePopup => {
                debug!("Closing popup for {} [#{}]", name, id);

                let popup = read_lock!(popup);
                popup.hide();
            }
        }

        Continue(true)
    });
}

/// Takes a widget and adds it into a new `gtk::EventBox`.
/// The event box container is returned.
fn wrap_widget<W: IsA<Widget>>(widget: &W) -> EventBox {
    let container = EventBox::new();
    container.add_events(EventMask::SCROLL_MASK);
    container.add(widget);
    container
}

/// Configures the module's container according to the common config options.
fn setup_module_common_options(container: EventBox, common: CommonConfig) {
    common.show_if.map_or_else(
        || {
            container.show_all();
        },
        |show_if| {
            let script = Script::new_polling(show_if);
            let container = container.clone();
            let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            spawn(async move {
                script
                    .run(|(_, success)| {
                        send!(tx, success);
                    })
                    .await;
            });
            rx.attach(None, move |success| {
                if success {
                    container.show_all();
                } else {
                    container.hide();
                };
                Continue(true)
            });
        },
    );

    let left_click_script = common.on_click_left.map(Script::new_polling);
    let middle_click_script = common.on_click_middle.map(Script::new_polling);
    let right_click_script = common.on_click_right.map(Script::new_polling);

    container.connect_button_press_event(move |_, event| {
        let script = match event.button() {
            1 => left_click_script.as_ref(),
            2 => middle_click_script.as_ref(),
            3 => right_click_script.as_ref(),
            _ => None,
        };

        if let Some(script) = script {
            trace!("Running on-click script: {}", event.button());

            match await_sync(async { script.get_output().await }) {
                Ok((OutputStream::Stderr(out), _)) => error!("{out}"),
                Err(err) => error!("{err:?}"),
                _ => {}
            }
        }

        Inhibit(false)
    });

    let scroll_up_script = common.on_scroll_up.map(Script::new_polling);
    let scroll_down_script = common.on_scroll_down.map(Script::new_polling);

    container.connect_scroll_event(move |_, event| {
        println!("{:?}", event.direction());

        let script = match event.direction() {
            ScrollDirection::Up => scroll_up_script.as_ref(),
            ScrollDirection::Down => scroll_down_script.as_ref(),
            _ => None,
        };

        if let Some(script) = script {
            trace!("Running on-scroll script: {}", event.direction());

            match await_sync(async { script.get_output().await }) {
                Ok((OutputStream::Stderr(out), _)) => error!("{out}"),
                Err(err) => error!("{err:?}"),
                _ => {}
            }
        }

        Inhibit(false)
    });

    if let Some(tooltip) = common.tooltip {
        DynamicString::new(&tooltip, move |string| {
            container.set_tooltip_text(Some(&string));
            Continue(true)
        });
    }
}
