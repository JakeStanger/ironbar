use crate::config::CommonConfig;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use cairo::{Format, ImageSurface};
use glib::translate::IntoGlibPtr;
use glib::Propagation;
use gtk::prelude::*;
use gtk::DrawingArea;
use mlua::{Function, LightUserData, Lua};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tracing::error;

#[derive(Debug, Clone, Deserialize)]
pub struct CairoModule {
    path: PathBuf,

    #[serde(default = "default_frequency")]
    frequency: u64,

    #[serde(default = "default_size")]
    width: u32,
    #[serde(default = "default_size")]
    height: u32,

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

    fn name() -> &'static str {
        "lua"
    }

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        _context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: Receiver<Self::ReceiveMessage>,
    ) -> color_eyre::Result<()>
    where
        <Self as Module<gtk::Box>>::SendMessage: Clone,
    {
        // Lua needs to run synchronously with the GTK updates,
        // so the controller does not handle the script engine.
        Ok(())
    }

    fn into_widget(
        self,
        _context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> color_eyre::Result<ModuleParts<gtk::Box>>
    where
        <Self as Module<gtk::Box>>::SendMessage: Clone,
    {
        let container = gtk::Box::new(info.bar_position.orientation(), 0);

        let surface =
            ImageSurface::create(Format::ARgb32, self.width as i32, self.height as i32).unwrap();

        let area = DrawingArea::new();

        let lua = unsafe { Lua::unsafe_new() };
        lua.load(include_str!("../../lua/init.lua")).exec().unwrap();

        let script = fs::read_to_string(self.path).unwrap();

        area.connect_draw(move |_, cr| {
            let function: Function = lua.load(&script).eval().unwrap();

            cr.set_source_surface(&surface, 0.0, 0.0).unwrap();

            let ptr = unsafe { cr.clone().into_glib_ptr().cast() };

            if let Err(err) = function.call::<_, u32>(LightUserData(ptr)) {
                error!("{err}")
            }

            Propagation::Proceed
        });

        area.set_size_request(self.width as i32, self.height as i32);
        container.add(&area);

        glib::spawn_future_local(async move {
            loop {
                area.queue_draw();
                glib::timeout_future(Duration::from_millis(self.frequency)).await;
            }
        });

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}