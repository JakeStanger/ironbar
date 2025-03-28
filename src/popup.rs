use crate::clients::wayland::{OutputEvent, OutputEventType};
use crate::config::BarPosition;
use crate::gtk_helpers::{IronbarGtkExt, WidgetGeometry};
use crate::modules::{ModuleInfo, ModulePopupParts, PopupButton};
use crate::{Ironbar, glib_recv, rc_mut};
use glib::{Propagation, clone};
use gtk::prelude::*;
use gtk::{ApplicationWindow, Button, EventControllerMotion, Orientation, Widget, Window};
use gtk_layer_shell::{Edge, LayerShell};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::{debug, trace};

#[derive(Debug, Clone)]
pub struct PopupCacheValue {
    pub name: String,
    pub content: ModulePopupParts,
}

#[derive(Debug, Clone)]
pub struct Popup {
    pub window: ApplicationWindow,
    pub container_cache: Rc<RefCell<HashMap<usize, PopupCacheValue>>>,
    pub button_cache: Rc<RefCell<Vec<Button>>>,
    pos: BarPosition,
    current_widget: Rc<RefCell<Option<(usize, usize)>>>,
    output_size: Rc<RefCell<(i32, i32)>>,
}

impl Popup {
    /// Creates a new popup window.
    /// This includes setting up gtk-layer-shell
    /// and an empty `gtk::Box` container.
    pub fn new(
        ironbar: Rc<Ironbar>,
        module_info: &ModuleInfo,
        output_size: (i32, i32),
        gap: i32,
    ) -> Self {
        let pos = module_info.bar_position;
        let orientation = pos.orientation();

        let win = ApplicationWindow::builder()
            .application(module_info.app)
            .build();

        win.init_layer_shell();
        win.set_monitor(Some(module_info.monitor));
        win.set_layer(gtk_layer_shell::Layer::Overlay);
        win.set_namespace(Some(env!("CARGO_PKG_NAME")));

        win.set_margin(Edge::Top, if pos == BarPosition::Top { gap } else { 0 });
        win.set_margin(
            Edge::Bottom,
            if pos == BarPosition::Bottom { gap } else { 0 },
        );
        win.set_margin(Edge::Left, if pos == BarPosition::Left { gap } else { 0 });
        win.set_margin(Edge::Right, if pos == BarPosition::Right { gap } else { 0 });

        win.set_anchor(
            Edge::Top,
            pos == BarPosition::Top || orientation == Orientation::Vertical,
        );
        win.set_anchor(Edge::Bottom, pos == BarPosition::Bottom);
        win.set_anchor(
            Edge::Left,
            pos == BarPosition::Left || orientation == Orientation::Horizontal,
        );
        win.set_anchor(Edge::Right, pos == BarPosition::Right);

        let event_controller = EventControllerMotion::new();
        {
            let win = win.clone();
            event_controller.connect_leave(move |motion| {
                const THRESHOLD: f64 = 3.0;

                let ev = motion.current_event().unwrap();

                let w = win.size(Orientation::Horizontal);
                let h = win.size(Orientation::Vertical);

                let Some((x, y)) = ev.position() else { return };

                // some child widgets trigger this event
                // so check we're actually outside the window
                let hide = match pos {
                    BarPosition::Top => {
                        x < THRESHOLD
                            || y > f64::from(h) - THRESHOLD
                            || x > f64::from(w) - THRESHOLD
                    }
                    BarPosition::Bottom => {
                        x < THRESHOLD || y < THRESHOLD || x > f64::from(w) - THRESHOLD
                    }
                    BarPosition::Left => {
                        y < THRESHOLD
                            || x > f64::from(w) - THRESHOLD
                            || y > f64::from(h) - THRESHOLD
                    }
                    BarPosition::Right => {
                        y < THRESHOLD || x < THRESHOLD || y > f64::from(h) - THRESHOLD
                    }
                };

                if hide {
                    win.hide();
                }
            });
        }

        win.add_controller(event_controller);

        let output_size = rc_mut!(output_size);

        // respond to resolution changes
        {
            let output_size = output_size.clone();
            let output_name = module_info.output_name.to_string();

            let on_output_event = move |event: OutputEvent| {
                if event.event_type == OutputEventType::Update
                    && event.output.name.unwrap_or_default() == output_name
                {
                    *output_size.borrow_mut() = event.output.logical_size.unwrap_or_default();
                }
            };

            glib_recv!(
                ironbar.clients.borrow_mut().wayland().subscribe_outputs(),
                on_output_event
            );
        }

        Self {
            window: win,
            container_cache: rc_mut!(HashMap::new()),
            button_cache: rc_mut!(vec![]),
            pos,
            current_widget: rc_mut!(None),
            output_size,
        }
    }

    pub fn register_content(&self, key: usize, name: String, content: ModulePopupParts) {
        debug!("Registered popup content for #{}", key);

        for button in &content.buttons {
            button.ensure_popup_id();
        }

        let orientation = self.pos.orientation();
        let window = self.window.clone();

        let current_widget = self.current_widget.clone();
        let cache = self.container_cache.clone();
        let button_cache = self.button_cache.clone();

        let output_size = self.output_size.clone();

        window.connect_default_width_notify(|win| {
            println!("POPUP WIDTH CHANGE");
        });

        window.connect_width_request_notify(move |win| {
            println!("POPUP WIDTH REQUEST CHANGE");
        });

        // FIXME: connect_size_allocate has been removed - need to find replacement
        //  https://docs.gtk.org/gtk4/migrating-3to4.html#adapt-to-gtkwidgets-size-allocation-changes
        // content
        //     .container
        //     .connect_size_allocate(move |container, rect| {
        //         if container.is_visible() {
        //             trace!("Resized:  {}x{}", rect.width(), rect.height());
        //
        //             if let Some((widget_id, button_id)) = *current_widget.borrow() {
        //                 if let Some(PopupCacheValue { .. }) = cache.borrow().get(&widget_id) {
        //                     Self::set_position(
        //                         &button_cache.borrow(),
        //                         button_id,
        //                         orientation,
        //                         &window,
        //                         &output_size,
        //                     );
        //                 }
        //             }
        //         }
        //     });

        self.button_cache
            .borrow_mut()
            .append(&mut content.buttons.clone());

        self.container_cache
            .borrow_mut()
            .insert(key, PopupCacheValue { name, content });
    }

    pub fn show(&self, widget_id: usize, button_id: usize) {
        self.clear_window();

        if let Some(PopupCacheValue { content, .. }) = self.container_cache.borrow().get(&widget_id)
        {
            *self.current_widget.borrow_mut() = Some((widget_id, button_id));

            content.container.add_class("popup");
            self.window.set_child(Some(&content.container));
            self.window.show();

            Self::set_position(
                &self.button_cache.borrow(),
                button_id,
                self.pos.orientation(),
                &self.window,
                &self.output_size,
            );
        }
    }

    pub fn show_at(&self, widget_id: usize, geometry: WidgetGeometry) {
        self.clear_window();

        if let Some(PopupCacheValue { content, .. }) = self.container_cache.borrow().get(&widget_id)
        {
            content.container.add_class("popup");
            self.window.set_child(Some(&content.container));

            self.window.show();
            Self::set_pos(
                geometry,
                self.pos.orientation(),
                &self.window,
                *self.output_size.borrow(),
            );
        }
    }

    fn set_position(
        buttons: &[Button],
        button_id: usize,
        orientation: Orientation,
        window: &ApplicationWindow,
        output_size: &Rc<RefCell<(i32, i32)>>,
    ) {
        let button = buttons
            .iter()
            .find(|b| b.popup_id() == button_id)
            .expect("to find valid button");

        let geometry = button.geometry(orientation);
        println!("geometry: {geometry:?}");
        Self::set_pos(geometry, orientation, window, *output_size.borrow());
    }

    fn clear_window(&self) {
        self.window.set_child(None::<&gtk::Box>);
    }

    /// Hides the popup
    pub fn hide(&self) {
        *self.current_widget.borrow_mut() = None;
        self.window.hide();
    }

    /// Checks if the popup is currently visible
    pub fn visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn current_widget(&self) -> Option<usize> {
        self.current_widget.borrow().map(|w| w.0)
    }

    /// Sets the popup's X/Y position relative to the left or border of the screen
    /// (depending on orientation).
    fn set_pos(
        geometry: WidgetGeometry,
        orientation: Orientation,
        window: &ApplicationWindow,
        output_size: (i32, i32),
    ) {
        let edge = if orientation == Orientation::Horizontal {
            Edge::Left
        } else {
            Edge::Top
        };

        let screen_size = if orientation == Orientation::Horizontal {
            output_size.0
        } else {
            output_size.1
        };

        println!("screen: {screen_size:?}");

        let allocation = window.allocation();
        let popup_width = allocation.width();
        let popup_height = allocation.height();

        let popup_size = if orientation == Orientation::Horizontal {
            popup_width
        } else {
            popup_height
        };

        if popup_size == 0 {
            window.set_margin(edge, 0);
            return;
        }

        let widget_center = geometry.position + f64::from(geometry.size) / 2.0;

        let bar_offset = (f64::from(screen_size) - f64::from(geometry.bar_size)) / 2.0;

        let mut offset = bar_offset + (widget_center - (f64::from(popup_size) / 2.0)).round();

        if offset < 5.0 {
            offset = 5.0;
        } else if offset > f64::from(screen_size - popup_size) - 5.0 {
            offset = f64::from(screen_size - popup_size) - 5.0;
        }

        window.set_margin(edge, offset as i32);
    }
}
