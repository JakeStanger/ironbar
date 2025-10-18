use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::channels::SyncSenderExt;
use crate::clients::wayland;
use crate::{Ironbar, arc_mut, debug, get_display};
use gtk::gdk::Monitor;
use gtk::gdk::prelude::*;
use smithay_client_toolkit::output::OutputInfo;
use tokio::sync::broadcast;

/// There are two ways in which we are notified of output events:
/// 1. Through a wayland client event
/// 2. Through a GDK event
///
/// We need to collate both events before we can call `load_output_bars`.
#[derive(Debug, Clone)]
pub enum MonitorState {
    Disconnected,
    WaylandConnected(OutputInfo),
    GdkConnected(glib::SendWeakRef<Monitor>),
    BothConnected(OutputInfo, glib::SendWeakRef<Monitor>),
}

#[derive(Debug, Clone)]
pub struct MonitorEvent {
    pub connector: String,
    pub state: MonitorState,
    tx: broadcast::Sender<MonitorEvent>,
}

impl MonitorEvent {
    fn send(&self) {
        debug!("Monitor event: {self:?}");
        self.tx.send_expect(self.clone());
    }

    fn disconnect(&mut self) {
        self.state = MonitorState::Disconnected;

        self.send();
    }

    fn connect_wayland(&mut self, wl_monitor: &OutputInfo) {
        self.state = match &self.state {
            MonitorState::GdkConnected(gdk_monitor) => {
                MonitorState::BothConnected(wl_monitor.clone(), gdk_monitor.clone())
            }
            _ => MonitorState::WaylandConnected(wl_monitor.clone()),
        };

        self.send();
    }

    fn connect_gdk(&mut self, gdk_monitor: glib::SendWeakRef<Monitor>) {
        self.state = match &self.state {
            MonitorState::WaylandConnected(wl_monitor) => {
                MonitorState::BothConnected(wl_monitor.clone(), gdk_monitor)
            }
            _ => MonitorState::GdkConnected(gdk_monitor),
        };

        self.send();
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct BroadcastChannel<T>(broadcast::Sender<T>, Arc<Mutex<broadcast::Receiver<T>>>);

impl<T> From<(broadcast::Sender<T>, broadcast::Receiver<T>)> for BroadcastChannel<T> {
    fn from(value: (broadcast::Sender<T>, broadcast::Receiver<T>)) -> Self {
        Self(value.0, arc_mut!(value.1))
    }
}

#[derive(Debug)]
pub struct Service {
    // Associates a connector ID (e.g. HDMI-1) to a state
    output_channel: BroadcastChannel<MonitorEvent>,
}

impl Service {
    pub(crate) fn new(ironbar: &Rc<Ironbar>) -> Self {
        let output_channel = broadcast::channel(32);

        let mut rx_wl_outputs = ironbar.clients.borrow_mut().wayland().subscribe_outputs();

        let monitors = Arc::new(Mutex::new(HashMap::new()));

        // listen to wayland events
        {
            let monitors = monitors.clone();
            let output_tx = output_channel.0.clone();

            glib::spawn_future_local(async move {
                while let Ok(event) = rx_wl_outputs.recv().await {
                    debug!("Wayland output event: {event:?}");
                    if let Some(name) = &event.output.name {
                        let mut guard = monitors.lock().unwrap();
                        let entry = guard.entry(name.clone()).or_insert_with(|| MonitorEvent {
                            connector: name.clone(),
                            state: MonitorState::Disconnected,
                            tx: output_tx.clone(),
                        });
                        match event.event_type {
                            wayland::OutputEventType::New => entry.connect_wayland(&event.output),
                            wayland::OutputEventType::Destroyed => entry.disconnect(),
                            wayland::OutputEventType::Update => {}
                        };
                    }
                }
            });
        }

        // listen to GDK events
        {
            let output_tx = output_channel.0.clone();
            let monitors = monitors.clone();

            let display = get_display();
            display
                .monitors()
                .connect_items_changed(move |list, position, removed, added| {
                    debug!("GDK event: +{added}, -{removed}");
                    for added_idx in position..position + added {
                        if let Some(monitor) = list.item(added_idx).and_downcast::<Monitor>() {
                            let output_tx = output_tx.clone();
                            let monitors = monitors.clone();

                            /*
                             * At this point, we have a `gdk::Monitor`. However, the object initially
                             * has all its fields `None`, including `connector`.
                             *
                             * We have to listen for the notify event for the `connector` being set.
                             */
                            monitor.connect_notify(Some("connector"), move |m, _| {
                                if let Some(connector) = m.connector() {
                                    monitors
                                        .lock()
                                        .unwrap()
                                        .entry(connector.to_string())
                                        .or_insert_with(|| MonitorEvent {
                                            connector: connector.to_string(),
                                            state: MonitorState::Disconnected,
                                            tx: output_tx.clone(),
                                        })
                                        .connect_gdk(glib::SendWeakRef::from(
                                            ObjectExt::downgrade(m),
                                        ));
                                }
                            });
                        }
                    }
                });
        }

        Self {
            output_channel: output_channel.into(),
        }
    }

    /// Subscribes to events
    pub fn subscribe(&self) -> broadcast::Receiver<MonitorEvent> {
        self.output_channel.0.subscribe()
    }
}
