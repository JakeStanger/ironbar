use crate::bar::create_bar;
use crate::channels::{AsyncSenderExt, MpscReceiverExt};
use crate::config::diff::{BarDiff, ConfigDiff, Diff, MonitorDiff};
use crate::config::{BarConfig, Config, MonitorConfig};
use crate::{Ironbar, find_monitor_for_output, load_output_bars_for, spawn};
use gtk::Application;
use gtk::gdk::Monitor;
use notify::event::{DataChange, ModifyKind};
use notify::{Event, EventKind, Result, Watcher, recommended_watcher};
use smithay_client_toolkit::output::OutputInfo;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use tracing::{debug, error, info};

pub fn install(path: &Path, app: &Application, ironbar: &Rc<Ironbar>) {
    let (tx, rx) = tokio::sync::mpsc::channel(8);

    spawn({
        let path = path.to_path_buf();
        let dir_path = path
            .parent()
            .expect("parent path should exist")
            .to_path_buf();

        async move {
            let is_conf_change = move |event: &Event| {
                event
                    .paths
                    .first()
                    .is_some_and(|p| p.file_stem() == path.file_stem())
                    && event.kind == EventKind::Modify(ModifyKind::Data(DataChange::Any))
            };

            let watcher = recommended_watcher(move |res: Result<Event>| match res {
                Ok(event) if is_conf_change(&event) => {
                    tx.send_spawn(());
                }
                Ok(_) => {}
                Err(err) => error!("{err:?}"),
            });

            let Ok(mut watcher) = watcher else {
                error!("failed to start config watcher");
                return;
            };

            if let Err(err) = watcher.watch(&dir_path, notify::RecursiveMode::NonRecursive) {
                error!("{err:?}");
            }

            // avoid watcher from dropping
            loop {
                tokio::time::sleep(core::time::Duration::from_secs(1)).await;
            }
        }
    });

    let wl = ironbar.clients.borrow_mut().wayland();
    let outputs = wl.output_info_all();

    let ironbar = ironbar.clone();

    rx.recv_glib(app, move |app, ()| {
        let old_config = <RefCell<Config> as Clone>::clone(&ironbar.config).into_inner();

        ironbar.reload_config();

        let new_config = ironbar.config.clone();

        let diff = ConfigDiff::diff(&old_config, &new_config.borrow());

        // TODO: handle other changes (icon theme, ...)

        let apply_bar_diff = {
            let ironbar = ironbar.clone();

            move |diff: BarDiff,
                  position: usize,
                  bar_config: BarConfig,
                  mon_name: &str,
                  monitor: &Monitor| {
                let mut bars = ironbar.bars.borrow_mut();
                let Some(bar) = bars.get_mut(position) else {
                    error!("failed to find bar for monitor {mon_name}");
                    return;
                };

                debug!(
                    "updating single bar for {mon_name} (bar={}) | {diff:?}",
                    bar.name()
                );

                match diff {
                    BarDiff::Recreate => {
                        info!("recreating bar {}", bar.name());

                        let new_bar = create_bar(
                            app,
                            monitor,
                            mon_name.to_string(),
                            bar_config,
                            ironbar.clone(),
                        );

                        let old = std::mem::replace(bar, new_bar);
                        old.close();
                    }
                    BarDiff::Reload(diff) => {
                        info!("hot-reloading bar {}", bar.name());

                        bar.apply_diff(diff, bar_config, monitor);
                    }
                }
            }
        };

        if let Diff::Updated(bar_diff) = diff.bar_diff {
            let config = &new_config.borrow().bar;

            for output in &outputs {
                let mon_name = output.name.clone().unwrap_or_default();

                let Some(monitor) = find_monitor_for_output(output) else {
                    error!("failed to find matching monitor for {mon_name}");
                    continue;
                };

                let Some(position) = ironbar
                    .bars
                    .borrow()
                    .iter()
                    .position(|b| b.monitor_name() == mon_name)
                else {
                    error!("failed to find bar for monitor {mon_name}");
                    continue;
                };

                apply_bar_diff(
                    bar_diff.clone(),
                    position,
                    config.clone(),
                    &mon_name,
                    &monitor,
                );
            }

            // TODO - apply diff to every monitor
        }

        if let Diff::Updated(monitor_diff) = diff.monitor_diff {
            let add = |output: &OutputInfo, monitor: &Monitor| match load_output_bars_for(
                &ironbar, app, output, monitor,
            ) {
                Ok(mut bars) => ironbar.bars.borrow_mut().append(&mut bars),
                Err(err) => error!("{err:?}"),
            };

            let remove = |name: &str| {
                let to_remove = ironbar.bars_by_monitor_name(name);

                ironbar
                    .bars
                    .borrow_mut()
                    .retain(|b| b.monitor_name() != name);

                for bar in to_remove {
                    bar.close();
                }
            };

            for added in monitor_diff.added {
                let Some(output) = outputs.iter().find(|&output| {
                    output.name.clone().unwrap_or_default() == added
                        || output
                            .description
                            .clone()
                            .unwrap_or_default()
                            .starts_with(&added)
                }) else {
                    error!("failed to find output for '{added}'");
                    continue;
                };

                let Some(monitor) = find_monitor_for_output(output) else {
                    error!("failed to find matching monitor for {added}");
                    continue;
                };

                add(output, &monitor);
            }

            for removed in monitor_diff.removed {
                remove(&removed);
            }

            for (mon_name, diff) in monitor_diff.updated {
                let Some(output) = outputs.iter().find(|&output| {
                    output.name.clone().unwrap_or_default() == mon_name
                        || output
                            .description
                            .clone()
                            .unwrap_or_default()
                            .starts_with(&mon_name)
                }) else {
                    error!("failed to find output for '{mon_name}'");
                    continue;
                };

                let Some(monitor) = find_monitor_for_output(output) else {
                    error!("failed to find matching monitor for {mon_name}");
                    continue;
                };

                match diff {
                    MonitorDiff::Recreate => {
                        info!("Recreating all bars for {mon_name}");
                        remove(&mon_name);
                        add(output, &monitor);
                    }
                    MonitorDiff::UpdateSingle(diff) => {
                        let Some(position) = ironbar
                            .bars
                            .borrow()
                            .iter()
                            .position(|b| b.monitor_name() == mon_name)
                        else {
                            error!("failed to find bar for monitor {mon_name}");
                            continue;
                        };

                        let Some(bar_config) = new_config
                            .borrow()
                            .monitors
                            .as_ref()
                            .and_then(|monitors| monitors.get(&mon_name))
                            .map(|config| match config {
                                MonitorConfig::Single(config) => config.clone(),
                                MonitorConfig::Multiple(_) => unreachable!(),
                            })
                        else {
                            error!("failed to find config for bar on {mon_name}");
                            return;
                        };

                        apply_bar_diff(diff, position, bar_config, &mon_name, &monitor);
                    }
                    MonitorDiff::UpdateMultiple(diffs) => {
                        debug!("Updating multiple bars for {mon_name} | {diffs:?}");

                        let positions = ironbar
                            .bars
                            .borrow()
                            .iter()
                            .enumerate()
                            .filter_map(|(i, bar)| {
                                if bar.monitor_name() == mon_name {
                                    Some(i)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();

                        for (i, (diff, position)) in diffs.into_iter().zip(positions).enumerate() {
                            let Some(bar_config) = new_config
                                .borrow()
                                .monitors
                                .as_ref()
                                .and_then(|monitors| monitors.get(&mon_name))
                                .and_then(|config| match config {
                                    MonitorConfig::Single(_) => unreachable!(),
                                    MonitorConfig::Multiple(configs) => configs.get(i).cloned(),
                                })
                            else {
                                error!("failed to find config for bar on {mon_name}");
                                return;
                            };

                            apply_bar_diff(diff, position, bar_config, &mon_name, &monitor);
                        }
                    }
                }
            }
        }
    });
}
