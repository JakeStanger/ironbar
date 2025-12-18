use crate::config::{MarqueeMode, MarqueeOnHover, TruncateMode};
use glib::ControlFlow;
use glib::{SignalHandlerId, markup_escape_text};
use gtk::gdk::{BUTTON_MIDDLE, BUTTON_PRIMARY, BUTTON_SECONDARY, Paintable};
use gtk::glib;
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{
    EventControllerMotion, EventSequenceState, GestureClick, Label, ScrolledWindow, Snapshot,
    Widget,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum MouseButton {
    Any,
    Primary = BUTTON_PRIMARY,
    Middle = BUTTON_MIDDLE,
    Secondary = BUTTON_SECONDARY,
}

pub trait IronbarGtkExt {
    /// Gets a data tag on a widget, if it exists.
    fn get_tag<V: 'static>(&self, key: &str) -> Option<&V>;
    /// Sets a data tag on a widget.
    fn set_tag<V: 'static>(&self, key: &str, value: V);

    /// Returns an iterator for the widget's first-level children.
    fn children(&self) -> ChildIterator;

    /// Adds a `GestureClick` controller with a `connect_pressed` signal callback.
    /// A mouse button can be specified to filter click events.
    fn connect_pressed<F>(&self, button: MouseButton, f: F) -> SignalHandlerId
    where
        F: Fn() + 'static;

    /// Adds a `GestureClick` controller with separate handlers for single and double clicks.
    /// Single-click is delayed by the double-click timeout (250ms) to distinguish from double-clicks.
    /// If `on_double` is None, behaves like `connect_pressed` with no delay.
    fn connect_pressed_with_double_click<F1, F2>(
        &self,
        button: MouseButton,
        on_single: F1,
        on_double: Option<F2>,
    ) -> SignalHandlerId
    where
        F1: Fn() + 'static,
        F2: Fn() + 'static;
}

impl<W: IsA<Widget>> IronbarGtkExt for W {
    fn get_tag<V: 'static>(&self, key: &str) -> Option<&V> {
        unsafe { self.data(key).map(|val| val.as_ref()) }
    }

    fn set_tag<V: 'static>(&self, key: &str, value: V) {
        unsafe { self.set_data(key, value) }
    }

    fn children(&self) -> ChildIterator {
        ChildIterator::new(self)
    }

    fn connect_pressed<F>(&self, button: MouseButton, f: F) -> SignalHandlerId
    where
        F: Fn() + 'static,
    {
        let controller = GestureClick::new();

        if button != MouseButton::Any {
            controller.set_button(button as u32);
        }

        let id = controller.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(EventSequenceState::Claimed);
            f();
        });

        self.add_controller(controller);
        id
    }

    fn connect_pressed_with_double_click<F1, F2>(
        &self,
        button: MouseButton,
        on_single: F1,
        on_double: Option<F2>,
    ) -> SignalHandlerId
    where
        F1: Fn() + 'static,
        F2: Fn() + 'static,
    {
        let controller = GestureClick::new();

        if button != MouseButton::Any {
            controller.set_button(button as u32);
        }

        // If no double-click handler provided, behave like regular connect_pressed
        let Some(on_double) = on_double else {
            let id = controller.connect_pressed(move |gesture, _, _, _| {
                gesture.set_state(EventSequenceState::Claimed);
                on_single();
            });
            self.add_controller(controller);
            return id;
        };

        // Wrap callbacks in Rc to make them cloneable
        let on_single = Rc::new(on_single);
        let on_double = Rc::new(on_double);

        // Track pending single-click timeout to cancel if double-click occurs
        let pending_single_click: Rc<Cell<Option<glib::SourceId>>> = Rc::new(Cell::new(None));

        let id = controller.connect_pressed(move |gesture, n_press, _, _| {
            gesture.set_state(EventSequenceState::Claimed);

            match n_press {
                1 => {
                    // Single click: delay action to see if double-click comes
                    let on_single = on_single.clone();
                    let pending = pending_single_click.clone();

                    // Cancel any existing pending single-click
                    if let Some(source_id) = pending.take() {
                        source_id.remove();
                    }

                    // Schedule single-click action after double-click timeout
                    let timeout_ms = crate::config::get_double_click_time_ms();
                    let source_id = glib::timeout_add_local_once(
                        Duration::from_millis(timeout_ms),
                        move || {
                            on_single();
                            pending.set(None);
                        },
                    );

                    pending_single_click.set(Some(source_id));
                }
                2 => {
                    // Double click: cancel pending single-click and execute double-click action
                    if let Some(source_id) = pending_single_click.take() {
                        source_id.remove();
                    }

                    on_double();
                }
                _ => {
                    // Ignore triple-clicks and beyond
                }
            }
        });

        self.add_controller(controller);
        id
    }
}

pub struct ChildIterator {
    curr: Option<Widget>,
}

impl ChildIterator {
    fn new<W: IsA<Widget>>(parent: &W) -> Self {
        Self {
            curr: parent.first_child(),
        }
    }
}

impl Iterator for ChildIterator {
    type Item = Widget;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr.clone();
        let next = curr.as_ref().and_then(WidgetExt::next_sibling);
        self.curr.clone_from(&next);
        curr // return current rather than next to include first child
    }
}

pub trait IronbarLabelExt {
    /// Sets the label value to the provided string.
    ///
    /// If the label does not contain markup `span` tags,
    /// the text is escaped to avoid issues with special characters (ie `&`).
    /// Otherwise, the text is used verbatim, and it is up to the user to escape.
    fn set_label_escaped(&self, label: &str);

    fn truncate(&self, mode: TruncateMode);
}

impl IronbarLabelExt for Label {
    fn set_label_escaped(&self, label: &str) {
        if label.contains("<span") {
            self.set_label(label);
        } else {
            self.set_label(&markup_escape_text(label));
        }
    }

    fn truncate(&self, mode: TruncateMode) {
        self.set_ellipsize(<TruncateMode as Into<EllipsizeMode>>::into(mode));

        if let Some(length) = mode.length() {
            self.set_width_chars(length);
        }

        if let Some(length) = mode.max_length() {
            self.set_max_width_chars(length);
        }
    }
}

pub trait IronbarPaintableExt {
    /// Scales a `Paintable`. to the requested size,
    /// returning a new `Paintable`.
    ///
    /// Aspect ratio is preserved.
    fn scale(self, target_width: f64, target_height: f64) -> Option<Paintable>;
}

impl<T> IronbarPaintableExt for T
where
    T: IsA<Paintable>,
{
    fn scale(self, target_width: f64, target_height: f64) -> Option<Paintable> {
        let ratio = self.intrinsic_aspect_ratio();

        let (width, height) = if ratio > 1.0 {
            (target_width, target_width / ratio)
        } else {
            (target_height * ratio, target_height)
        };

        let snapshot = Snapshot::new();
        self.snapshot(&snapshot, width, height);
        snapshot.to_paintable(None)
    }
}

/// Calculates the pixel width of a string given the label it's displayed in.
fn pixel_width(label: &gtk::Label, string: &str) -> i32 {
    let layout = label.create_pango_layout(Some(string));
    let (w, _) = layout.size(); // in Pango units (1/1024 px)
    w / gtk::pango::SCALE // back to integer pixels
}

/// Wrapper around a `Label` that applies either truncation or marquee scrolling.
///
/// The widget returned by [`widget`] can be packed directly into layouts. The underlying
/// `Label` can be accessed via [`label`] to apply CSS classes or other label-specific settings.
pub struct OverflowLabel {
    label: Label,
    widget: Widget,
    marquee: Option<MarqueeLabel>,
}

impl OverflowLabel {
    /// Create a new overflow-aware label.
    ///
    /// If `truncate` is provided, marquee is ignored. If marquee is enabled, the label
    /// is wrapped in a `ScrolledWindow` that handles scrolling logic.
    pub fn new(label: Label, truncate: Option<TruncateMode>, marquee_mode: MarqueeMode) -> Self {
        if truncate.is_some() || !marquee_mode.enable {
            if let Some(truncate) = truncate {
                label.truncate(truncate);
            }

            let widget = label.clone().upcast::<Widget>();

            Self {
                label,
                widget,
                marquee: None,
            }
        } else {
            let marquee = MarqueeLabel::new(label.clone(), marquee_mode);
            let widget = marquee.widget().clone().upcast::<Widget>();

            Self {
                label,
                widget,
                marquee: Some(marquee),
            }
        }
    }

    pub fn label(&self) -> &Label {
        &self.label
    }

    pub fn widget(&self) -> &Widget {
        &self.widget
    }

    /// Set the label text, escaping markup unless the string already contains `<span`.
    /// When marquee is enabled, this also resets scroll state for the new text.
    pub fn set_label_escaped(&self, text: &str) {
        if let Some(marquee) = &self.marquee {
            marquee.set_text(text);
        } else {
            self.label.set_label_escaped(text);
        }
    }
}

struct MarqueeLabel {
    inner: Rc<MarqueeInner>,
    scrolled: ScrolledWindow,
}

struct MarqueeInner {
    label: Label,
    mode: MarqueeMode,
    text: RefCell<String>,
    original_text_width: Cell<i32>,
    is_hovered: Cell<bool>,
    pause_started_at: Cell<Option<Instant>>,
    is_scrolling: Cell<bool>,
    reset_at_cached: Cell<Option<f64>>,
    pause_duration: Duration,
}

impl MarqueeLabel {
    fn new(label: Label, marquee_mode: MarqueeMode) -> Self {
        let scrolled = ScrolledWindow::builder()
            .vscrollbar_policy(gtk::PolicyType::Never)
            .build();

        scrolled.hscrollbar().set_visible(false);
        scrolled.set_child(Some(&label));

        let inner = Rc::new(MarqueeInner {
            label,
            mode: marquee_mode.clone(),
            text: RefCell::new(String::new()),
            original_text_width: Cell::new(0),
            is_hovered: Cell::new(false),
            pause_started_at: Cell::new(None),
            is_scrolling: Cell::new(false),
            reset_at_cached: Cell::new(None),
            pause_duration: Duration::from_millis(marquee_mode.pause_duration),
        });

        {
            let inner = inner.clone();
            let scrolled_clone = scrolled.clone();

            scrolled.add_tick_callback(move |_, _| inner.tick(&scrolled_clone));
        }

        if marquee_mode.on_hover != MarqueeOnHover::None {
            let motion_controller = EventControllerMotion::new();

            {
                let inner = inner.clone();
                motion_controller.connect_enter(move |_, _, _| {
                    inner.is_hovered.set(true);
                });
            }

            {
                let inner = inner.clone();
                motion_controller.connect_leave(move |_| {
                    inner.is_hovered.set(false);
                });
            }

            scrolled.add_controller(motion_controller);
        }

        Self { inner, scrolled }
    }

    fn widget(&self) -> &ScrolledWindow {
        &self.scrolled
    }

    fn set_text(&self, text: &str) {
        self.inner.text.replace(text.to_string());
        self.inner
            .original_text_width
            .set(pixel_width(&self.inner.label, text));
        self.inner.is_scrolling.set(false);
        self.inner.reset_at_cached.set(None);
        self.inner.pause_started_at.set(None);

        let hadjustment = self.scrolled.hadjustment();
        hadjustment.set_value(0.0);

        if let Some(max_length) = self.inner.mode.max_length {
            let sample_string = text.chars().take(max_length as usize).collect::<String>();
            let width = pixel_width(&self.inner.label, &sample_string);
            self.scrolled.set_min_content_width(width);
        } else {
            self.scrolled.set_min_content_width(0);
        }

        self.inner.label.set_label_escaped(text);
    }
}

impl MarqueeInner {
    fn tick(&self, scrolled: &ScrolledWindow) -> ControlFlow {
        let allocated_width = scrolled.width();
        let text = self.text.borrow();

        let needs_scroll = self.original_text_width.get() > allocated_width;

        if needs_scroll {
            if !self.is_scrolling.get() {
                let duplicated_text = format!("{}{}{}", &*text, &self.mode.separator, &*text);
                self.label.set_label(&duplicated_text);

                let reset_at =
                    pixel_width(&self.label, &format!("{}{}", &*text, &self.mode.separator)) as f64;
                self.reset_at_cached.set(Some(reset_at));

                self.pause_started_at.set(Some(Instant::now()));
                self.is_scrolling.set(true);
            }

            let reset_at = self
                .reset_at_cached
                .get()
                .expect("reset_at is always set before is_scrolling becomes true");

            let is_paused = if let Some(start_time) = self.pause_started_at.get() {
                start_time.elapsed() <= self.pause_duration
            } else {
                false
            };

            if is_paused {
                return ControlFlow::Continue;
            }

            if self.pause_started_at.get().is_some() {
                self.pause_started_at.set(None);
            }

            let should_scroll = match self.mode.on_hover {
                MarqueeOnHover::Play => self.is_hovered.get(),
                MarqueeOnHover::Pause => !self.is_hovered.get(),
                MarqueeOnHover::None => true,
            };

            if should_scroll {
                let hadjustment = scrolled.hadjustment();
                let v = hadjustment.value() + self.mode.scroll_speed;
                if v >= reset_at {
                    hadjustment.set_value(v - reset_at);
                    self.pause_started_at.set(Some(Instant::now()));
                } else {
                    hadjustment.set_value(v);
                }
            }
        } else if self.is_scrolling.get() {
            self.label.set_label_escaped(&text);
            scrolled.hadjustment().set_value(0.0);
            self.is_scrolling.set(false);
            self.reset_at_cached.set(None);
            self.pause_started_at.set(None);
        }

        ControlFlow::Continue
    }
}
