use super::Response;
use crate::bar::Bar;
use crate::ipc::{BarCommand, BarCommandType};
use crate::modules::PopupButton;
use crate::Ironbar;
use std::rc::Rc;

pub fn handle_command(command: BarCommand, ironbar: &Rc<Ironbar>) -> Response {
    use BarCommandType::*;

    let bars = ironbar.bars_by_name(&command.name);
    if bars.is_empty() {
        return Response::error("Invalid bar name");
    }

    let responses = bars
        .into_iter()
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
        .collect::<Vec<_>>();

    responses
        .into_iter()
        .reduce(|acc, rsp| match (acc, rsp) {
            // If all responses are Ok, return one Ok. We assume we'll never mix Ok and OkValue.
            (Response::Ok, _) => Response::Ok,
            // Two or more OkValues create a multi:
            (Response::OkValue { value: v1 }, Response::OkValue { value: v2 }) => Response::Multi {
                values: vec![v1, v2],
            },
            (Response::Multi { mut values }, Response::OkValue { value: v }) => {
                values.push(v);
                Response::Multi { values }
            }
            _ => unreachable!(),
        })
        .unwrap()
}

fn set_visible(bar: &Bar, visible: bool) -> Response {
    bar.set_visible(visible);
    Response::Ok
}

fn show_popup(bar: &Bar, widget_name: &str) -> Response {
    let popup = bar.popup();

    // only one popup per bar, so hide if open for another widget
    popup.hide();

    let data = popup
        .container_cache
        .borrow()
        .iter()
        .find(|(_, value)| value.name == widget_name)
        .map(|(id, value)| (*id, value.content.buttons.first().cloned()));

    match data {
        Some((id, Some(button))) => {
            let button_id = button.popup_id();
            popup.show(id, button_id);

            Response::Ok
        }
        Some((_, None)) => Response::error("Module has no popup functionality"),
        None => Response::error("Invalid module name"),
    }
}

fn hide_popup(bar: &Bar) -> Response {
    let popup = bar.popup();
    popup.hide();

    Response::Ok
}
