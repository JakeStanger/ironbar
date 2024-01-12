use crate::config::{BarPosition, MarginConfig, ModuleConfig};
use crate::modules::{
    create_module, set_widget_identifiers, wrap_widget, ModuleInfo, ModuleLocation,
};
use crate::popup::Popup;
use crate::{Config, Ironbar};
use color_eyre::Result;
use glib::Propagation;
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, IconTheme, Orientation, Window, WindowType};
use gtk_layer_shell::LayerShell;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use tracing::{debug, info};

#[derive(Debug, Clone)]
enum Inner {
    New { config: Option<Config> },
    Loaded { popup: Rc<RefCell<Popup>> },
}

#[derive(Debug, Clone)]
pub struct Bar {
    name: String,
    monitor_name: String,
    position: BarPosition,

    ironbar: Rc<Ironbar>,

    window: ApplicationWindow,

    content: gtk::Box,

    start: gtk::Box,
    center: gtk::Box,
    end: gtk::Box,

    inner: Inner,
}

impl Bar {
    pub fn new(
        app: &Application,
        monitor_name: String,
        config: Config,
        ironbar: Rc<Ironbar>,
    ) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .type_(WindowType::Toplevel)
            .build();

        let name = config
            .name
            .clone()
            .unwrap_or_else(|| format!("bar-{}", Ironbar::unique_id()));

        window.set_widget_name(&name);

        let position = config.position;
        let orientation = position.get_orientation();

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

        window.add(&content);

        window.connect_destroy_event(|_, _| {
            info!("Shutting down");
            gtk::main_quit();
            Propagation::Proceed
        });

        Bar {
            name,
            monitor_name,
            position,
            ironbar,
            window,
            content,
            start,
            center,
            end,
            inner: Inner::New {
                config: Some(config),
            },
        }
    }

    pub fn init(mut self, monitor: &Monitor) -> Result<Self> {
        let Inner::New { ref mut config } = self.inner else {
            return Ok(self);
        };

        let Some(config) = config.take() else {
            return Ok(self);
        };

        info!(
            "Initializing bar '{}' on '{}'",
            self.name, self.monitor_name
        );

        self.setup_layer_shell(
            &self.window,
            true,
            config.anchor_to_edges,
            config.margin,
            monitor,
        );

        let start_hidden = config.start_hidden.unwrap_or(config.autohide.is_some());

        if let Some(autohide) = config.autohide {
            let hotspot_window = Window::new(WindowType::Toplevel);

            Self::setup_autohide(&self.window, &hotspot_window, autohide);
            self.setup_layer_shell(
                &hotspot_window,
                false,
                config.anchor_to_edges,
                config.margin,
                monitor,
            );

            if start_hidden {
                hotspot_window.show();
            }
        }

        let load_result = self.load_modules(config, monitor)?;

        self.show(!start_hidden);

        self.inner = Inner::Loaded {
            popup: load_result.popup,
        };
        Ok(self)
    }

    /// Sets up GTK layer shell for a provided application window.
    fn setup_layer_shell(
        &self,
        win: &impl IsA<Window>,
        exclusive_zone: bool,
        anchor_to_edges: bool,
        margin: MarginConfig,
        monitor: &Monitor,
    ) {
        let position = self.position;

        win.init_layer_shell();
        win.set_monitor(monitor);
        win.set_layer(gtk_layer_shell::Layer::Top);
        win.set_namespace(env!("CARGO_PKG_NAME"));

        if exclusive_zone {
            win.auto_exclusive_zone_enable();
        }

        win.set_layer_shell_margin(gtk_layer_shell::Edge::Top, margin.top);
        win.set_layer_shell_margin(gtk_layer_shell::Edge::Bottom, margin.bottom);
        win.set_layer_shell_margin(gtk_layer_shell::Edge::Left, margin.left);
        win.set_layer_shell_margin(gtk_layer_shell::Edge::Right, margin.right);

        let bar_orientation = position.get_orientation();

        win.set_anchor(
            gtk_layer_shell::Edge::Top,
            position == BarPosition::Top
                || (bar_orientation == Orientation::Vertical && anchor_to_edges),
        );
        win.set_anchor(
            gtk_layer_shell::Edge::Bottom,
            position == BarPosition::Bottom
                || (bar_orientation == Orientation::Vertical && anchor_to_edges),
        );
        win.set_anchor(
            gtk_layer_shell::Edge::Left,
            position == BarPosition::Left
                || (bar_orientation == Orientation::Horizontal && anchor_to_edges),
        );
        win.set_anchor(
            gtk_layer_shell::Edge::Right,
            position == BarPosition::Right
                || (bar_orientation == Orientation::Horizontal && anchor_to_edges),
        );
    }

    fn setup_autohide(window: &ApplicationWindow, hotspot_window: &Window, timeout: u64) {
        hotspot_window.hide();

        hotspot_window.set_opacity(0.0);
        hotspot_window.set_decorated(false);
        hotspot_window.set_size_request(0, 1);

        {
            let hotspot_window = hotspot_window.clone();

            window.connect_leave_notify_event(move |win, _| {
                let win = win.clone();
                let hotspot_window = hotspot_window.clone();

                glib::timeout_add_local_once(Duration::from_millis(timeout), move || {
                    win.hide();
                    hotspot_window.show();
                });
                Propagation::Proceed
            });
        }

        {
            let win = window.clone();

            hotspot_window.connect_enter_notify_event(move |hotspot_win, _| {
                hotspot_win.hide();
                win.show();

                Propagation::Proceed
            });
        }
    }

    /// Loads the configured modules onto a bar.
    fn load_modules(&self, config: Config, monitor: &Monitor) -> Result<BarLoadResult> {
        let icon_theme = IconTheme::new();
        if let Some(ref theme) = config.icon_theme {
            icon_theme.set_custom_theme(Some(theme));
        }

        let app = &self.window.application().expect("to exist");

        macro_rules! info {
            ($location:expr) => {
                ModuleInfo {
                    app,
                    bar_position: config.position,
                    monitor,
                    output_name: &self.monitor_name,
                    location: $location,
                    icon_theme: &icon_theme,
                }
            };
        }

        // popup ignores module location so can bodge this for now
        let popup = Popup::new(&info!(ModuleLocation::Left), config.popup_gap);
        let popup = Rc::new(RefCell::new(popup));

        if let Some(modules) = config.start {
            let info = info!(ModuleLocation::Left);
            add_modules(&self.start, modules, &info, &self.ironbar, &popup)?;
        }

        if let Some(modules) = config.center {
            let info = info!(ModuleLocation::Center);
            add_modules(&self.center, modules, &info, &self.ironbar, &popup)?;
        }

        if let Some(modules) = config.end {
            let info = info!(ModuleLocation::Right);
            add_modules(&self.end, modules, &info, &self.ironbar, &popup)?;
        }

        let result = BarLoadResult { popup };

        Ok(result)
    }

    fn show(&self, include_window: bool) {
        debug!("Showing bar: {}", self.name);

        // show each box but do not use `show_all`.
        // this ensures `show_if` option works as intended.
        self.start.show();
        self.center.show();
        self.end.show();
        self.content.show();

        if include_window {
            self.window.show();
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn popup(&self) -> Rc<RefCell<Popup>> {
        match &self.inner {
            Inner::New { .. } => {
                panic!("Attempted to get popup of uninitialized bar. This is a serious bug!")
            }
            Inner::Loaded { popup } => popup.clone(),
        }
    }
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

#[derive(Debug)]
struct BarLoadResult {
    popup: Rc<RefCell<Popup>>,
}

/// Adds modules into a provided GTK box,
/// which should be one of its left, center or right containers.
fn add_modules(
    content: &gtk::Box,
    modules: Vec<ModuleConfig>,
    info: &ModuleInfo,
    ironbar: &Rc<Ironbar>,
    popup: &Rc<RefCell<Popup>>,
) -> Result<()> {
    let orientation = info.bar_position.get_orientation();

    macro_rules! add_module {
        ($module:expr, $id:expr) => {{
            let common = $module.common.take().expect("common config to exist");
            let widget_parts = create_module(
                *$module,
                $id,
                ironbar.clone(),
                common.name.clone(),
                &info,
                &Rc::clone(&popup),
            )?;
            set_widget_identifiers(&widget_parts, &common);

            let container = wrap_widget(&widget_parts.widget, common, orientation);
            content.add(&container);
        }};
    }

    for config in modules {
        let id = Ironbar::unique_id();
        match config {
            #[cfg(feature = "clipboard")]
            ModuleConfig::Clipboard(mut module) => add_module!(module, id),
            #[cfg(feature = "clock")]
            ModuleConfig::Clock(mut module) => add_module!(module, id),
            ModuleConfig::Custom(mut module) => add_module!(module, id),
            #[cfg(feature = "focused")]
            ModuleConfig::Focused(mut module) => add_module!(module, id),
            ModuleConfig::Label(mut module) => add_module!(module, id),
            #[cfg(feature = "launcher")]
            ModuleConfig::Launcher(mut module) => add_module!(module, id),
            #[cfg(feature = "music")]
            ModuleConfig::Music(mut module) => add_module!(module, id),
            ModuleConfig::Script(mut module) => add_module!(module, id),
            #[cfg(feature = "sys_info")]
            ModuleConfig::SysInfo(mut module) => add_module!(module, id),
            #[cfg(feature = "tray")]
            ModuleConfig::Tray(mut module) => add_module!(module, id),
            #[cfg(feature = "upower")]
            ModuleConfig::Upower(mut module) => add_module!(module, id),
            #[cfg(feature = "workspaces")]
            ModuleConfig::Workspaces(mut module) => add_module!(module, id),
        }
    }

    Ok(())
}

pub fn create_bar(
    app: &Application,
    monitor: &Monitor,
    monitor_name: String,
    config: Config,
    ironbar: Rc<Ironbar>,
) -> Result<Bar> {
    let bar = Bar::new(app, monitor_name, config, ironbar);
    bar.init(monitor)
}
