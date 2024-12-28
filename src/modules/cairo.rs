use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::config::CommonConfig;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};
use cairo::{Format, ImageSurface};
use glib::translate::IntoGlibPtr;
use glib::Propagation;
use gtk::prelude::*;
use gtk::DrawingArea;
use mlua::{Error, Function, LightUserData};
use notify::event::ModifyKind;
use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time::sleep;
use tracing::{debug, error};

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
    #[serde(default = "default_frequency")]
    frequency: u64,

    /// The canvas width in pixels.
    ///
    /// **Default**: `42`
    #[serde(default = "default_size")]
    width: u32,

    /// The canvas height in pixels.
    ///
    /// **Default**: `42`
    #[serde(default = "default_size")]
    height: u32,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

const fn default_size() -> u32 {
    42
}

const fn default_frequency() -> u64 {
    200
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
        let id = context.id.to_string();

        let container = gtk::Box::new(info.bar_position.orientation(), 0);

        let surface = ImageSurface::create(Format::ARgb32, self.width as i32, self.height as i32)?;

        let area = DrawingArea::new();

        let lua = context
            .ironbar
            .clients
            .borrow_mut()
            .lua(&context.ironbar.config_dir);

        // this feels kinda dirty,
        // but it keeps draw functions separate in the global scope
        let script = fs::read_to_string(&self.path)?
            .replace("function draw", format!("function __draw_{id}").as_str());
        lua.load(&script).exec()?;

        {
            let lua = lua.clone();
            let id = id.clone();

            let path = self.path.clone();

            area.connect_draw(move |_, cr| {
                let function: Function = lua
                    .load(include_str!("../../lua/draw.lua"))
                    .eval()
                    .expect("to be valid");

                if let Err(err) = cr.set_source_surface(&surface, 0.0, 0.0) {
                    error!("{err}");
                    return Propagation::Stop;
                }

                let ptr = unsafe { cr.clone().into_glib_ptr().cast() };

                // mlua needs a valid return type, even if we don't return anything
                if let Err(err) =
                    function.call::<_, Option<bool>>((id.as_str(), LightUserData(ptr)))
                {
                    if let Error::RuntimeError(message) = err {
                        let message = message.split_once("]:").expect("to exist").1;
                        error!("[lua runtime error] {}:{message}", path.display());
                    } else {
                        error!("{err}");
                    }

                    return Propagation::Stop;
                }

                Propagation::Proceed
            });
        }

        area.set_size_request(self.width as i32, self.height as i32);
        container.add(&area);

        glib::spawn_future_local(async move {
            loop {
                area.queue_draw();
                glib::timeout_future(Duration::from_millis(self.frequency)).await;
            }
        });

        context.subscribe().recv_glib(move |_ev| {
            let res = fs::read_to_string(&self.path)
                .map(|s| s.replace("function draw", format!("function __draw_{id}").as_str()));

            match res {
                Ok(script) => match lua.load(&script).exec() {
                    Ok(()) => {}
                    Err(Error::SyntaxError { message, .. }) => {
                        let message = message.split_once("]:").expect("to exist").1;
                        error!("[lua syntax error] {}:{message}", self.path.display());
                    }
                    Err(err) => error!("lua error: {err:?}"),
                },
                Err(err) => error!("{err:?}"),
            }
        });

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}
