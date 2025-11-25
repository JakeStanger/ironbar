use super::Response;
use crate::Ironbar;
use crate::bar::Bar;
use crate::ipc::{BarCommand, BarCommandType};
use glib::prelude::Cast;
use gtk::Button;
use std::rc::Rc;

pub fn handle_command(command: &BarCommand, ironbar: &Rc<Ironbar>) -> Response {
    use BarCommandType::*;

    let bars = ironbar.bars_by_name(&command.name);

    bars.into_iter()
        .map(|bar| match &command.subcommand {
            Show => set_visible(&bar, true),
            Hide => set_visible(&bar, false),
            SetVisible { visible } => set_visible(&bar, *visible),
            ToggleVisible => set_visible(&bar, !bar.visible()),
            GetVisible => Response::OkValue {
                value: bar.visible().to_string(),
            },
            ShowPopup { widget_name } => show_popup(&bar, widget_name),
            HidePopup => hide_popup(&bar),
            SetPopupVisible {
                widget_name,
                visible,
            } => {
                if *visible {
                    show_popup(&bar, widget_name)
                } else {
                    hide_popup(&bar)
                };
                Response::Ok
            }
            TogglePopup { widget_name } => {
                if bar.popup().visible() {
                    hide_popup(&bar)
                } else {
                    show_popup(&bar, widget_name)
                };
                Response::Ok
            }
            GetPopupVisible => Response::OkValue {
                value: bar.popup().visible().to_string(),
            },
            SetExclusive { exclusive } => {
                bar.set_exclusive(*exclusive);
                Response::Ok
            }
        })
        .reduce(|acc, rsp| match (acc, rsp) {
            // If all responses are `Ok`, return one `Ok`. We assume we'll never mix `Ok` and `OkValue`.
            (Response::Ok, _) => Response::Ok,
            // Two or more `OkValue`s create a multi:
            (Response::OkValue { value: v1 }, Response::OkValue { value: v2 }) => Response::Multi {
                values: vec![v1, v2],
            },
            (Response::Multi { mut values }, Response::OkValue { value: v }) => {
                values.push(v);
                Response::Multi { values }
            }
            (acc, _) => acc,
        })
        .unwrap_or(Response::error("Invalid bar name"))
}

fn set_visible(bar: &Bar, visible: bool) -> Response {
    bar.set_visible(visible);
    Response::Ok
}

fn show_popup(bar: &Bar, widget_name: &str) -> Response {
    let popup = bar.popup();

    // only one popup per bar, so hide if open for another widget
    popup.hide();

    let module_ref = bar.modules().iter().find(|m| m.name == widget_name);

    let module_button = module_ref.and_then(|m| m.widget.downcast_ref::<Button>());

    match (module_ref, module_button) {
        (Some(module_ref), Some(button)) => {
            if popup.show_for(module_ref.id, button) {
                Response::Ok
            } else {
                Response::error("Module has no popup functionality")
            }
        }
        (Some(_), None) => Response::error("Module has no popup functionality"),
        (None, _) => Response::error("Invalid module name"),
    }
}

fn hide_popup(bar: &Bar) -> Response {
    let popup = bar.popup();
    popup.hide();

    Response::Ok
}
