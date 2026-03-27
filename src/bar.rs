use crate::config::diff::BarDiffDetails;
use crate::config::{BarConfig, BarPosition, MarginConfig, ModuleConfig};
use crate::modules::{BarModuleFactory, ModuleInfo, ModuleLocation, ModuleRef};
use crate::popup::Popup;
use crate::{Ironbar, rc_mut};
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, CenterBox, EventControllerMotion, Orientation, Window};
use gtk_layer_shell::LayerShell;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
enum Inner {
    New {
        config: Option<BarConfig>,
    },
    Loaded {
        module_refs: Vec<ModuleRef>,
        popup: Rc<Popup>,
        instance: Rc<Bar>,
    },
}

#[derive(Debug, Clone)]
struct AutohideState {
    hotspot_window: Window,
    lock_state: LockState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockState {
    /// No lock held on bar visibility
    Unlocked,
    /// Lock held on bar visibility
    Locked,
    /// Lock held on bar visibility
    /// with request to hide bar as soon as lock is cleared
    LockedPendingClose,
}

#[derive(Debug, Clone)]
pub struct Bar {
    name: String,
    monitor_name: String,
    position: BarPosition,

    ironbar: Rc<Ironbar>,

    window: ApplicationWindow,

    content: CenterBox,

    start: gtk::Box,
    center: gtk::Box,
    end: gtk::Box,

    autohide_state: Rc<RefCell<Option<AutohideState>>>,

    inner: Inner,
}

impl Bar {
    pub fn new(
        app: &Application,
        monitor_name: String,
        config: BarConfig,
        ironbar: Rc<Ironbar>,
    ) -> Self {
        let window = ApplicationWindow::builder().application(app).build();

        let name = config
            .name
            .clone()
            .unwrap_or_else(|| format!("bar-{}", Ironbar::unique_id()));

        window.set_widget_name(&name);

        let position = config.position;
        let orientation = position.orientation();

        let content = CenterBox::builder().orientation(orientation).name("bar");

        let content = if orientation == Orientation::Horizontal {
            content.height_request(config.height)
        } else {
            content.width_request(config.height)
        }
        .build();

        content.add_css_class("container");

        let start = create_container("start", orientation);
        let center = create_container("center", orientation);
        let end = create_container("end", orientation);

        window.set_child(Some(&content));

        Self {
            name,
            monitor_name,
            position,
            ironbar,
            window,
            content,
            start,
            center,
            end,
            autohide_state: rc_mut!(None),
            inner: Inner::New {
                config: Some(config),
            },
        }
    }

    pub fn init(mut self, monitor: &Monitor) -> Self {
        let Inner::New { ref mut config } = self.inner else {
            return self;
        };

        let Some(config) = config.take() else {
            return self;
        };

        info!(
            "Initializing bar '{}' on '{}'",
            self.name, self.monitor_name
        );

        let start_hidden = config
            .start_hidden
            .unwrap_or_else(|| config.autohide.is_some());

        self.setup_layer_shell(
            &self.window,
            config.exclusive_zone.unwrap_or(!start_hidden),
            config.anchor_to_edges,
            config.margin,
            config.layer,
            monitor,
        );

        let autohide = config.autohide;
        let anchor_to_edges = config.anchor_to_edges;
        let margin = config.margin;

        let instance = Rc::new(self.clone());
        let load_result = self.load_modules(&instance, config, monitor);

        let autohide_state = if let Some(autohide) = autohide {
            let hotspot_window = Window::new();
            self.setup_autohide(&hotspot_window, load_result.popup.clone(), autohide);
            self.setup_layer_shell(
                &hotspot_window,
                false,
                anchor_to_edges,
                margin,
                gtk_layer_shell::Layer::Top,
                monitor,
            );

            if start_hidden {
                hotspot_window.set_visible(true);
            }

            Some(AutohideState {
                lock_state: LockState::Unlocked,
                hotspot_window,
            })
        } else {
            None
        };

        self.show(!start_hidden);

        *self.autohide_state.borrow_mut() = autohide_state;
        self.inner = Inner::Loaded {
            popup: load_result.popup,
            module_refs: load_result.module_refs,
            instance,
        };

        self
    }

    /// Closes the bar, consuming it.
    pub fn close(self) {
        self.window.close();
        self.window.destroy();
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name of the output the bar is displayed on.
    pub fn monitor_name(&self) -> &str {
        &self.monitor_name
    }

    pub fn popup(&self) -> Rc<Popup> {
        match &self.inner {
            Inner::New { .. } => {
                panic!("Attempted to get popup of uninitialized bar. This is a serious bug!")
            }
            Inner::Loaded { popup, .. } => popup.clone(),
        }
    }

    pub fn visible(&self) -> bool {
        self.window.is_visible()
    }

    /// Sets the window visibility status
    pub fn set_visible(&self, visible: bool) {
        self.window.set_visible(visible);
    }

    pub fn set_exclusive(&self, exclusive: bool) {
        if exclusive {
            self.window.auto_exclusive_zone_enable();
        } else {
            self.window.set_exclusive_zone(0);
        }
    }

    pub fn set_locked(&self, locked: bool) {
        let mut autohide_state = self.autohide_state.borrow_mut();
        let Some(autohide_state) = autohide_state.as_mut() else {
            return;
        };

        if locked {
            autohide_state.lock_state = LockState::Locked;
        } else if autohide_state.lock_state == LockState::LockedPendingClose {
            self.window.set_visible(false);
            autohide_state.hotspot_window.set_visible(true);
            autohide_state.lock_state = LockState::Unlocked;
        }
    }

    pub fn modules(&self) -> &[ModuleRef] {
        match &self.inner {
            Inner::New { .. } => {
                panic!("Attempted to get modules of uninitialized bar. This is a serious bug!")
            }
            Inner::Loaded { module_refs, .. } => module_refs,
        }
    }

    pub fn apply_diff(&mut self, diff: BarDiffDetails, config: BarConfig, monitor: &Monitor) {
        let module_factory =
            BarModuleFactory::new(self.ironbar.clone(), self.instance(), self.popup()).into();
        let app = &self.window.application().expect("to exist");

        macro_rules! info {
            ($location:expr) => {
                ModuleInfo {
                    app,
                    bar_position: config.position,
                    monitor,
                    output_name: &self.monitor_name,
                    location: $location,
                }
            };
        }

        let module_refs = match &mut self.inner {
            Inner::New { .. } => {
                panic!("Attempted to get modules of uninitialized bar. This is a serious bug!")
            }
            Inner::Loaded { module_refs, .. } => module_refs,
        };

        let mut load_modules = |mut modules: Vec<ModuleConfig>,
                                diffs: Vec<usize>,
                                container: &gtk::Box,
                                location: ModuleLocation| {
            // avoid potential panic if all modules are disabled, but...
            if modules.is_empty() {
                return;
            }

            for i in diffs.into_iter().rev() {
                let Some(existing_module) = module_refs
                    .iter_mut()
                    .filter(|m| m.location == location)
                    .nth(i)
                else {
                    error!("failed to find existing module to replace");
                    return;
                };

                #[allow(unreachable_code)] // ...above check doesn't satisfy rustc
                let module = modules.remove(i);

                match module.create(&module_factory, container, &info!(location)) {
                    Ok(module) => {
                        let existing_wrapper = existing_module
                            .root_widget
                            .parent()
                            .expect("root should have revealer parent");
                        let new_wrapper = module
                            .root_widget
                            .parent()
                            .expect("root should have revealer parent");

                        container.reorder_child_after(&new_wrapper, Some(&existing_wrapper));
                        container.remove(&existing_wrapper);
                        let _ = std::mem::replace(existing_module, module);
                    }
                    Err(err) => error!("{err:?}"),
                }
            }
        };

        if let Some(modules) = config.start {
            load_modules(modules, diff.start, &self.start, ModuleLocation::Start);
        }

        if let Some(modules) = config.center {
            load_modules(modules, diff.center, &self.center, ModuleLocation::Center);
        }

        if let Some(modules) = config.end {
            load_modules(modules, diff.end, &self.end, ModuleLocation::End);
        }
    }

    /// Sets up GTK layer shell for a provided application window.
    fn setup_layer_shell(
        &self,
        win: &impl IsA<Window>,
        exclusive_zone: bool,
        anchor_to_edges: bool,
        margin: MarginConfig,
        layer: gtk_layer_shell::Layer,
        monitor: &Monitor,
    ) {
        use gtk_layer_shell::Edge;

        let position = self.position;

        win.init_layer_shell();
        win.set_monitor(Some(monitor));
        win.set_layer(layer);
        win.set_namespace(Some(env!("CARGO_PKG_NAME")));

        if exclusive_zone {
            win.auto_exclusive_zone_enable();
        }

        win.set_margin(Edge::Top, margin.top);
        win.set_margin(Edge::Bottom, margin.bottom);
        win.set_margin(Edge::Left, margin.left);
        win.set_margin(Edge::Right, margin.right);

        let bar_orientation = position.orientation();

        win.set_anchor(
            Edge::Top,
            position == BarPosition::Top
                || (bar_orientation == Orientation::Vertical && anchor_to_edges),
        );
        win.set_anchor(
            Edge::Bottom,
            position == BarPosition::Bottom
                || (bar_orientation == Orientation::Vertical && anchor_to_edges),
        );
        win.set_anchor(
            Edge::Left,
            position == BarPosition::Left
                || (bar_orientation == Orientation::Horizontal && anchor_to_edges),
        );
        win.set_anchor(
            Edge::Right,
            position == BarPosition::Right
                || (bar_orientation == Orientation::Horizontal && anchor_to_edges),
        );
    }

    fn setup_autohide(&self, hotspot_window: &Window, popup: Rc<Popup>, timeout: u64) {
        hotspot_window.set_visible(false);

        hotspot_window.set_opacity(0.01);
        hotspot_window.set_decorated(false);

        let (w, h) = match self.position {
            BarPosition::Top | BarPosition::Bottom => (0, 5),
            BarPosition::Left | BarPosition::Right => (5, 0),
        };
        hotspot_window.set_default_size(w, h);

        let timeout_id = rc_mut!(None);

        {
            let hotspot_window = hotspot_window.clone();
            let timeout_id = timeout_id.clone();

            let event_controller = EventControllerMotion::new();

            // TODO: Lots of repeated code and reference cloning going on -
            //  can we try and tidy this up?

            {
                let win = self.window.clone();
                let hotspot_window = hotspot_window.clone();
                let timeout_id = timeout_id.clone();
                let autohide_state = self.autohide_state.clone();

                popup.popover.connect_hide(move |popover| {
                    let win = win.clone();
                    let hotspot_window = hotspot_window.clone();
                    let tid = timeout_id.clone();
                    let popover = popover.clone();
                    let autohide_state = autohide_state.clone();

                    *timeout_id.borrow_mut() = Some(glib::timeout_add_local_once(
                        Duration::from_millis(timeout),
                        move || {
                            if !popover.is_visible() {
                                let mut autohide_state = autohide_state.borrow_mut();
                                if autohide_state.as_ref().map(|state| state.lock_state)
                                    == Some(LockState::Locked)
                                {
                                    autohide_state.as_mut().expect("to have value").lock_state =
                                        LockState::LockedPendingClose;
                                } else {
                                    win.set_visible(false);
                                    hotspot_window.set_visible(true);
                                }
                            }

                            *tid.borrow_mut() = None;
                        },
                    ));
                });
            }

            {
                let win = self.window.clone();
                let hotspot_window = hotspot_window.clone();
                let timeout_id = timeout_id.clone();
                let autohide_state = self.autohide_state.clone();

                event_controller.connect_leave(move |_| {
                    let win = win.clone();
                    let hotspot_window = hotspot_window.clone();
                    let tid = timeout_id.clone();
                    let popup = popup.clone();
                    let autohide_state = autohide_state.clone();

                    *timeout_id.borrow_mut() = Some(glib::timeout_add_local_once(
                        Duration::from_millis(timeout),
                        move || {
                            if !popup.visible() {
                                let mut autohide_state = autohide_state.borrow_mut();
                                if autohide_state.as_ref().map(|state| state.lock_state)
                                    == Some(LockState::Locked)
                                {
                                    autohide_state.as_mut().expect("to have value").lock_state =
                                        LockState::LockedPendingClose;
                                } else {
                                    win.set_visible(false);
                                    hotspot_window.set_visible(true);
                                }
                            }

                            *tid.borrow_mut() = None;
                        },
                    ));
                });
            }

            event_controller.connect_enter(move |_, _, _| {
                if let Some(id) = timeout_id.borrow_mut().take() {
                    id.remove();
                }
            });

            self.window.add_controller(event_controller);
        }

        {
            let win = self.window.clone();

            let event_controller = EventControllerMotion::new();

            let hotspot_win = hotspot_window.clone();
            event_controller.connect_enter(move |_, _, _| {
                hotspot_win.set_visible(false);
                win.set_visible(true);
            });

            hotspot_window.add_controller(event_controller);
        }
    }

    /// Loads the configured modules onto a bar.
    fn load_modules(
        &self,
        instance: &Rc<Bar>,
        config: BarConfig,
        monitor: &Monitor,
    ) -> BarLoadResult {
        let app = &self.window.application().expect("to exist");

        macro_rules! info {
            ($location:expr) => {
                ModuleInfo {
                    app,
                    bar_position: config.position,
                    monitor,
                    output_name: &self.monitor_name,
                    location: $location,
                }
            };
        }

        // popup ignores module location so can bodge this for now
        let popup = Popup::new(
            &info!(ModuleLocation::Start),
            config.popup_gap,
            config.popup_autohide,
        );
        let popup = Rc::new(popup);

        let mut refs = vec![];

        if let Some(modules) = config.start {
            self.content.set_start_widget(Some(&self.start));

            let info = info!(ModuleLocation::Start);
            refs.extend(add_modules(
                &self.start,
                modules,
                &info,
                &self.ironbar,
                instance,
                &popup,
            ));
        }

        if let Some(modules) = config.center {
            self.content.set_center_widget(Some(&self.center));

            let info = info!(ModuleLocation::Center);
            refs.extend(add_modules(
                &self.center,
                modules,
                &info,
                &self.ironbar,
                instance,
                &popup,
            ));
        }

        if let Some(modules) = config.end {
            self.content.set_end_widget(Some(&self.end));

            let info = info!(ModuleLocation::End);
            refs.extend(add_modules(
                &self.end,
                modules,
                &info,
                &self.ironbar,
                instance,
                &popup,
            ));
        }

        BarLoadResult {
            popup,
            module_refs: refs,
        }
    }

    fn show(&self, include_window: bool) {
        debug!("Showing bar: {}", self.name);

        // show each box but do not use `show_all`.
        // this ensures `show_if` option works as intended.
        self.start.set_visible(true);
        self.center.set_visible(true);
        self.end.set_visible(true);
        self.content.set_visible(true);

        if include_window {
            self.window.set_visible(true);
        }
    }

    fn instance(&self) -> Rc<Bar> {
        match &self.inner {
            Inner::New { .. } => {
                panic!("Attempted to get modules of uninitialized bar. This is a serious bug!")
            }
            Inner::Loaded { instance, .. } => instance.clone(),
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

    container.add_css_class("container");
    container
}

#[derive(Debug)]
struct BarLoadResult {
    popup: Rc<Popup>,
    module_refs: Vec<ModuleRef>,
}

/// Adds modules into a provided GTK box,
/// which should be one of its left, center or right containers.
fn add_modules(
    content: &gtk::Box,
    modules: Vec<ModuleConfig>,
    info: &ModuleInfo,
    ironbar: &Rc<Ironbar>,
    slf: &Rc<Bar>,
    popup: &Rc<Popup>,
) -> Vec<ModuleRef> {
    let module_factory = BarModuleFactory::new(ironbar.clone(), slf.clone(), popup.clone()).into();

    let mut results = vec![];
    for config in modules {
        let name = config.name();
        match config.create(&module_factory, content, info) {
            Ok(res) => results.push(res),
            Err(err) => error!("failed to create module {name}: {:?}", err),
        }
    }

    results
}

pub fn create_bar(
    app: &Application,
    monitor: &Monitor,
    monitor_name: String,
    config: BarConfig,
    ironbar: Rc<Ironbar>,
) -> Bar {
    let bar = Bar::new(app, monitor_name, config, ironbar);
    bar.init(monitor)
}
