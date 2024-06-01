use crate::ipc::commands::IronvarCommand;
use crate::ipc::Response;
use crate::{read_lock, write_lock, Ironbar};

pub fn handle_command(command: IronvarCommand) -> Response {
    match command {
        IronvarCommand::Set { key, value } => {
            let variable_manager = Ironbar::variable_manager();
            let mut variable_manager = write_lock!(variable_manager);
            match variable_manager.set(key, value) {
                Ok(()) => Response::Ok,
                Err(err) => Response::error(&format!("{err}")),
            }
        }
        IronvarCommand::Get { key } => {
            let variable_manager = Ironbar::variable_manager();
            let value = read_lock!(variable_manager).get(&key);
            match value {
                Some(value) => Response::OkValue { value },
                None => Response::error("Variable not found"),
            }
        }
        IronvarCommand::List => {
            let variable_manager = Ironbar::variable_manager();

            let mut values = read_lock!(variable_manager)
                .get_all()
                .iter()
                .map(|(k, v)| format!("{k}: {}", v.get().unwrap_or_default()))
                .collect::<Vec<_>>();

            values.sort();
            let value = values.join("\n");

            Response::OkValue { value }
        }
    }
}
