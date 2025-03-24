use crate::Ironbar;
use crate::bar::Bar;
use crate::ipc::{Response, StyleCommand};
use crate::modules::ModuleRef;
use crate::style::load_css;
use gtk::prelude::*;

pub fn handle_command(command: StyleCommand, ironbar: &Ironbar) -> Response {
    match command {
        StyleCommand::LoadCss { path } => {
            if path.exists() {
                load_css(path);
                Response::Ok
            } else {
                Response::error("File not found")
            }
        }
        StyleCommand::AddClass { module_name, name } => {
            let bars = ironbar.bars.borrow();
            let modules = modules_by_name(&bars, &module_name);

            if modules.is_empty() {
                return Response::error("Module not found");
            }

            for module in modules {
                module.add_css_class(&name);
            }

            Response::Ok
        }
        StyleCommand::RemoveClass { module_name, name } => {
            let bars = ironbar.bars.borrow();
            let modules = modules_by_name(&bars, &module_name);

            if modules.is_empty() {
                return Response::error("Module not found");
            }

            for module in modules {
                module.remove_css_class(&name);
            }

            Response::Ok
        }
        StyleCommand::ToggleClass { module_name, name } => {
            let bars = ironbar.bars.borrow();
            let modules = modules_by_name(&bars, &module_name);

            if modules.is_empty() {
                return Response::error("Module not found");
            }

            for module in modules {
                if module.widget.has_css_class(&name) {
                    module.remove_css_class(&name);
                } else {
                    module.add_css_class(&name);
                }
            }

            Response::Ok
        }
    }
}

fn modules_by_name<'a>(bars: &'a [Bar], name: &str) -> Vec<&'a ModuleRef> {
    bars.iter()
        .flat_map(Bar::modules)
        .filter(|w| w.name == name)
        .collect::<Vec<_>>()
}

impl ModuleRef {
    fn add_css_class(&self, name: &str) {
        self.widget.add_css_class(name);

        if let Some(ref popup) = self.popup {
            popup.add_css_class(name);
        }
    }

    fn remove_css_class(&self, name: &str) {
        self.widget.remove_css_class(name);

        if let Some(ref popup) = self.popup {
            popup.remove_css_class(name);
        }
    }
}
