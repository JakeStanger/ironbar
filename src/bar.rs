use crate::config::{BarPosition, ModuleConfig};
use crate::modules::{Module, ModuleInfo, ModuleLocation};
use crate::Config;
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Orientation};

pub fn create_bar(app: &Application, monitor: &Monitor, monitor_name: &str, config: Config) {
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

    load_modules(&left, &center, &right, app, config, monitor_name);
    win.add(&content);

    win.connect_destroy_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    win.show_all();
}

fn load_modules(
    left: &gtk::Box,
    center: &gtk::Box,
    right: &gtk::Box,
    app: &Application,
    config: Config,
    output_name: &str,
) {
    if let Some(modules) = config.left {
        let info = ModuleInfo {
            app,
            location: ModuleLocation::Left,
            bar_position: &config.position,
            output_name,
        };

        add_modules(left, modules, info);
    }

    if let Some(modules) = config.center {
        let info = ModuleInfo {
            app,
            location: ModuleLocation::Center,
            bar_position: &config.position,
            output_name,
        };

        add_modules(center, modules, info);
    }

    if let Some(modules) = config.right {
        let info = ModuleInfo {
            app,
            location: ModuleLocation::Right,
            bar_position: &config.position,
            output_name,
        };

        add_modules(right, modules, info);
    }
}

fn add_modules(content: &gtk::Box, modules: Vec<ModuleConfig>, info: ModuleInfo) {
    for config in modules {
        match config {
            ModuleConfig::Clock(module) => {
                let widget = module.into_widget(&info);
                widget.set_widget_name("clock");
                content.add(&widget);
            }
            ModuleConfig::Mpd(module) => {
                let widget = module.into_widget(&info);
                widget.set_widget_name("mpd");
                content.add(&widget);
            }
            ModuleConfig::Tray(module) => {
                let widget = module.into_widget(&info);
                widget.set_widget_name("tray");
                content.add(&widget);
            }
            ModuleConfig::Workspaces(module) => {
                let widget = module.into_widget(&info);
                widget.set_widget_name("workspaces");
                content.add(&widget);
            }
            ModuleConfig::SysInfo(module) => {
                let widget = module.into_widget(&info);
                widget.set_widget_name("sysinfo");
                content.add(&widget);
            }
            ModuleConfig::Launcher(module) => {
                let widget = module.into_widget(&info);
                widget.set_widget_name("launcher");
                content.add(&widget);
            }
            ModuleConfig::Script(module) => {
                let widget = module.into_widget(&info);
                widget.set_widget_name("script");
                content.add(&widget);
            }
            ModuleConfig::Focused(module) => {
                let widget = module.into_widget(&info);
                widget.set_widget_name("focused");
                content.add(&widget);
            }
        }
    }
}

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
