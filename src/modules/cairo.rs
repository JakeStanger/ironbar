use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::lua::LuaEngine;
use crate::config::{CommonConfig, ConfigLocation};
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};
use glib::translate::ToGlibPtr;
use gtk::DrawingArea;
use gtk::cairo::{Format, ImageSurface};
use gtk::prelude::*;
use mlua::{Error, Function, LightUserData, MetaMethod, Value};
use notify::event::ModifyKind;
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};
use serde::Deserialize;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time::sleep;
use tracing::{debug, error};

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct CairoModule {
    /// The path to the Lua script to load.
    /// This can be absolute, or relative to the working directory.
    ///
    /// The script must contain the entry `draw` function.
    ///
    /// **Required**
    path: PathBuf,

    /// The number of milliseconds between each draw call.
    ///
    /// **Default**: `200`
    frequency: u64,

    /// The canvas width in pixels.
    ///
    /// **Default**: `42`
    width: u32,

    /// The canvas height in pixels.
    ///
    /// **Default**: `42`
    height: u32,

    /// See [common options](module-level-options#common-options).
    pub common: Option<CommonConfig>,
}

impl Default for CairoModule {
    fn default() -> Self {
        Self {
            path: PathBuf::default(),
            frequency: 200,
            width: 42,
            height: 42,
            common: Some(CommonConfig::default()),
        }
    }
}

impl CairoModule {
    fn load_draw_function(&self, lua: &LuaEngine) -> Option<Value> {
        // Expect the script to return a drawing function/callable
        // In case of a syntax error we leave it empty for now
        let value = match lua.load(self.path.clone()).call::<Value>(()) {
            Ok(value) => value,
            Err(Error::SyntaxError { message, .. }) => {
                error!("[lua syntax error]: {message}");
                return None;
            }
            Err(Error::RuntimeError(message)) => {
                error!("[lua runtime error]: {message}");
                return None;
            }
            Err(err) => {
                error!("{err}");
                return None;
            }
        };

        // Check if the result is callable
        match &value {
            Value::Function(_) => Some(value),
            Value::Table(table) => {
                if let Some(metatable) = table.metatable()
                    && metatable
                        .contains_key(MetaMethod::Call.name())
                        .unwrap_or_default()
                {
                    Some(value)
                } else {
                    error!("[lua error]: {:?} is not callable", value);
                    None
                }
            }
            _ => {
                error!("[lua error]: Expected callable, but got {:?}", value);
                None
            }
        }
    }
}

impl Module<gtk::Box> for CairoModule {
    type SendMessage = ();
    type ReceiveMessage = ();

    module_impl!("cairo");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: Receiver<Self::ReceiveMessage>,
    ) -> color_eyre::Result<()>
    where
        <Self as Module<gtk::Box>>::SendMessage: Clone,
    {
        let path = self.path.clone();

        let tx = context.tx.clone();
        spawn(async move {
            let parent = path.parent().expect("to have parent path");

            let mut watcher = recommended_watcher({
                let path = path.clone();
                move |res: notify::Result<Event>| match res {
                    Ok(event) if matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) => {
                        debug!("{event:?}");

                        if event.paths.first().is_some_and(|p| p == &path) {
                            tx.send_update_spawn(());
                        }
                    }
                    Err(e) => error!("Error occurred when watching stylesheet: {:?}", e),
                    _ => {}
                }
            })
            .expect("Failed to create lua file watcher");

            watcher
                .watch(parent, RecursiveMode::NonRecursive)
                .expect("Failed to start lua file watcher");

            // avoid watcher from dropping
            loop {
                sleep(Duration::from_secs(1)).await;
            }
        });

        // Lua needs to run synchronously with the GTK updates,
        // so the controller does not handle the script engine.

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> color_eyre::Result<ModuleParts<gtk::Box>>
    where
        <Self as Module<gtk::Box>>::SendMessage: Clone,
    {
        let container = gtk::Box::new(info.bar_position.orientation(), 0);

        let surface = ImageSurface::create(Format::ARgb32, self.width as i32, self.height as i32)?;

        let area = DrawingArea::new();

        let config_dir = match &context.ironbar.config_location {
            ConfigLocation::Minimal | ConfigLocation::Desktop => {
                let path = ConfigLocation::default_path();
                path.parent().unwrap_or(&path).to_path_buf()
            }
            ConfigLocation::Custom(path) => path.parent().unwrap_or(path).to_path_buf(),
        };

        let lua = context.ironbar.clients.borrow_mut().lua(&config_dir);

        // Keep draw function in a mutex so it can be replaced on file change
        let draw_function = Rc::new(Mutex::new(self.load_draw_function(&lua)));

        {
            let draw_function = draw_function.clone();
            let draw_wrapper: Function = lua
                .load(include_str!("../../lua/draw.lua"))
                .eval()
                .expect("to be valid");

            area.set_draw_func(move |_, cr, w, h| {
                if let Err(err) = cr.set_source_surface(&surface, 0.0, 0.0) {
                    error!("{err}");
                    return;
                }

                let ptr = cr.to_glib_full();

                if let Some(ref current_draw_function) = *draw_function.lock().expect("Mutex lock")
                {
                    // mlua needs a valid return type, even if we don't return anything
                    if let Err(err) = draw_wrapper.call::<Option<bool>>((
                        current_draw_function,
                        LightUserData(ptr.cast()),
                        w,
                        h,
                    )) {
                        error!("lua error: {err}");
                    }
                }

                unsafe {
                    gtk::cairo::ffi::cairo_destroy(ptr);
                }
            });
        }

        area.set_size_request(self.width as i32, self.height as i32);
        container.append(&area);

        glib::spawn_future_local(async move {
            loop {
                area.queue_draw();
                glib::timeout_future(Duration::from_millis(self.frequency)).await;
            }
        });
        context.subscribe().recv_glib((), move |(), _ev| {
            // Reload/replace on file change
            if let Some(function) = self.load_draw_function(&lua) {
                draw_function.lock().expect("Mutex lock").replace(function);
            }
        });

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}
