use crate::config::{BarPosition, MarginConfig, ModuleConfig};
use crate::modules::{create_module, wrap_widget, ModuleInfo, ModuleLocation};
use crate::popup::Popup;
use crate::Config;
use color_eyre::Result;
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, IconTheme, Orientation};
use std::sync::{Arc, RwLock};
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

    setup_layer_shell(
        &win,
        monitor,
        config.position,
        config.anchor_to_edges,
        config.margin,
    );

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
    margin: MarginConfig,
) {
    gtk_layer_shell::init_for_window(win);
    gtk_layer_shell::set_monitor(win, monitor);
    gtk_layer_shell::set_layer(win, gtk_layer_shell::Layer::Top);
    gtk_layer_shell::auto_exclusive_zone_enable(win);
    gtk_layer_shell::set_namespace(win, env!("CARGO_PKG_NAME"));

    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Top, margin.top);
    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Bottom, margin.bottom);
    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Left, margin.left);
    gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Right, margin.right);

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
        add_modules(left, modules, &info, config.popup_gap)?;
    }

    if let Some(modules) = config.center {
        let info = info!(ModuleLocation::Center);
        add_modules(center, modules, &info, config.popup_gap)?;
    }

    if let Some(modules) = config.end {
        let info = info!(ModuleLocation::Right);
        add_modules(right, modules, &info, config.popup_gap)?;
    }

    Ok(())
}

/// Adds modules into a provided GTK box,
/// which should be one of its left, center or right containers.
fn add_modules(
    content: &gtk::Box,
    modules: Vec<ModuleConfig>,
    info: &ModuleInfo,
    popup_gap: i32,
) -> Result<()> {
    let popup = Popup::new(info, popup_gap);
    let popup = Arc::new(RwLock::new(popup));

    macro_rules! add_module {
        ($module:expr, $id:expr) => {{
            let common = $module.common.take().expect("Common config did not exist");
            let widget = create_module(*$module, $id, &info, &Arc::clone(&popup))?;
            let container = wrap_widget(&widget, common);
            content.add(&container);
        }};
    }

    for (id, config) in modules.into_iter().enumerate() {
        match config {
            #[cfg(feature = "clipboard")]
            ModuleConfig::Clipboard(mut module) => add_module!(module, id),
            #[cfg(feature = "clock")]
            ModuleConfig::Clock(mut module) => add_module!(module, id),
            ModuleConfig::Custom(mut module) => add_module!(module, id),
            ModuleConfig::Focused(mut module) => add_module!(module, id),
            ModuleConfig::Label(mut module) => add_module!(module, id),
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
