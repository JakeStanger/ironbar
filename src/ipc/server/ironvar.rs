use crate::Ironbar;
use crate::ipc::{IronvarCommand, Response};
use crate::ironvar::{Namespace, WritableNamespace};
use std::sync::Arc;

pub fn handle_command(command: IronvarCommand) -> Response {
    match command {
        IronvarCommand::Set { key, value } => {
            let variable_manager = Ironbar::variable_manager();
            match variable_manager.set(&key, value) {
                Ok(()) => Response::Ok,
                Err(err) => Response::error(&format!("{err}")),
            }
        }
        IronvarCommand::Get { mut key } => {
            let variable_manager = Ironbar::variable_manager();
            let mut ns: Arc<dyn Namespace + Sync + Send> = variable_manager;

            if key.contains('.') {
                for part in key.split('.') {
                    ns = if let Some(ns) = ns.get_namespace(part) {
                        ns.clone()
                    } else {
                        key = part.into();
                        break;
                    };
                }
            }

            let value = ns.get(&key);
            match value {
                Some(value) => Response::OkValue { value },
                None => Response::error("Variable not found"),
            }
        }
        IronvarCommand::List { namespace } => {
            let variable_manager = Ironbar::variable_manager();
            let mut ns: Arc<dyn Namespace + Sync + Send> = variable_manager;

            if let Some(namespace) = namespace {
                for part in namespace.split('.') {
                    ns = match ns.get_namespace(part) {
                        Some(ns) => ns.clone(),
                        None => return Response::error("Namespace not found"),
                    };
                }
            }

            let mut namespaces = ns
                .namespaces()
                .iter()
                .map(|ns| format!("<{ns}>"))
                .collect::<Vec<_>>();

            namespaces.sort();

            let mut value = namespaces.join("\n");

            let mut values = ns
                .get_all()
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>();

            values.sort();

            value.push('\n');
            value.push_str(&values.join("\n"));

            Response::OkValue { value }
        }
    }
}
