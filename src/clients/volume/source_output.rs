use std::sync::{Arc, Mutex};

use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SourceOutputInfo;
use libpulse_binding::context::subscribe::Operation;
use tokio::sync::broadcast;
use tracing::{debug, instrument};

use super::{ArcMutVec, Client, ConnectionState, Event, HasIndex, PulseObject, VolumeLevels};
use crate::lock;

#[derive(Debug, Clone)]
pub struct SourceOutput {
    pub index: u32,
    pub name: String,
    pub volume: VolumeLevels,
    pub muted: bool,

    pub can_set_volume: bool,
}

impl From<&SourceOutputInfo<'_>> for SourceOutput {
    fn from(value: &SourceOutputInfo) -> Self {
        Self {
            index: value.index,
            name: value
                .name
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            muted: value.mute,
            volume: value.volume.into(),
            can_set_volume: value.has_volume && value.volume_writable,
        }
    }
}

impl<'a> HasIndex for SourceOutputInfo<'a> {
    fn index(&self) -> u32 {
        self.index
    }
}

impl HasIndex for SourceOutput {
    fn index(&self) -> u32 {
        self.index
    }
}

impl<'a> PulseObject<'a> for SourceOutput {
    type Inner = SourceOutputInfo<'a>;

    #[inline]
    fn name(&self) -> String {
        self.name.clone()
    }
    #[inline]
    fn active(&self) -> bool {
        true
    }
    #[inline]
    fn set_active(&mut self, _active: bool) {}

    #[inline]
    fn add_event(info: Self) -> Event {
        Event::AddOutput(info)
    }
    #[inline]
    fn update_event(info: Self) -> Event {
        Event::UpdateOutput(info)
    }
    #[inline]
    fn remove_event(info: Self) -> Event {
        Event::RemoveOutput(info.index)
    }
}

impl Client {
    #[instrument(level = "trace")]
    pub fn source_outputs(&self) -> ArcMutVec<SourceOutput> {
        self.data.source_outputs.clone()
    }

    #[instrument(level = "trace")]
    pub fn set_output_volume(&self, index: u32, volume_percent: f64) {
        if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
            let Some(mut volume_levels) = ({
                let outputs = self.source_outputs();
                lock!(outputs).iter().find_map(|s| {
                    if s.index == index {
                        Some(s.volume.clone())
                    } else {
                        None
                    }
                })
            }) else {
                return;
            };

            volume_levels.set_percent(volume_percent);
            introspector.set_source_output_volume(index, &volume_levels.into(), None);
        }
    }

    #[instrument(level = "trace")]
    pub fn set_output_muted(&self, index: u32, muted: bool) {
        if let ConnectionState::Connected { introspector, .. } = &mut *lock!(self.connection) {
            introspector.set_source_output_mute(index, muted, None);
        }
    }
}

impl SourceOutput {
    pub(super) fn on_event(
        context: &Arc<Mutex<Context>>,
        outputs: &ArcMutVec<SourceOutput>,
        tx: &broadcast::Sender<Event>,
        op: Operation,
        i: u32,
    ) {
        let introspect = lock!(context).introspect();

        match op {
            Operation::New => {
                debug!("new source output");
                introspect.get_source_output_info(i, {
                    let outputs = outputs.clone();
                    let tx = tx.clone();

                    move |info| Self::add(info, &outputs, &tx)
                });
            }
            Operation::Changed => {
                debug!("source output changed");
                introspect.get_source_output_info(i, {
                    let outputs = outputs.clone();
                    let tx = tx.clone();

                    move |info| Self::update(info, &outputs, None, &tx)
                });
            }
            Operation::Removed => {
                debug!("source output removed");
                Self::remove(i, outputs, tx);
            }
        }
    }
}
