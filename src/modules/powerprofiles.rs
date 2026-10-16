use std::time::Duration;

use color_eyre::eyre::Result;
use gtk::{
    Button, Label,
    prelude::{BoxExt, ButtonExt, WidgetExt},
};
use serde::Deserialize;
use tokio::time;
use tracing::{error, trace};

use crate::{
    channels::{AsyncSenderExt, BroadcastReceiverExt},
    clients::powerprofiles,
    config::{CommonConfig, LayoutConfig, ProfileUpdateEvent, Profiles, State, default},
    image::IconLabel,
    module_impl,
    modules::{Module, ModuleParts},
    profiles, spawn,
};

impl State for Box<str> {
    fn matches(&self, value: &Self) -> bool {
        value == self
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct PowerProfilesProfile {
    icon: Box<str>,
}

impl PowerProfilesProfile {
    fn new(icon: &str) -> Self {
        Self {
            icon: Box::from(icon),
        }
    }
}

fn default_profiles() -> Profiles<Box<str>, PowerProfilesProfile> {
    profiles!(
        "performance":Box::from("performance") => PowerProfilesProfile::new("󱐋"),
        "balanced":Box::from("balanced") => PowerProfilesProfile::new(""),
        "power-saver":Box::from("power-saver") => PowerProfilesProfile::new("")
    )
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct PowerProfilesModule {
    /// See [profiles](profiles).
    #[serde(flatten)]
    profiles: Profiles<Box<str>, PowerProfilesProfile>,

    /// The format string to use for the widget button label.
    /// For available tokens, see [below](#formatting-tokens).
    ///
    /// **Default**: `{icon}`
    format: String,

    /// The icon to show on the bar widget button.
    /// Supports [image](images) icons.
    ///
    /// **Default**: `Power profile: {name}`
    tooltip_format: String,

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for PowerProfilesModule {
    fn default() -> Self {
        Self {
            profiles: Profiles::default(),
            format: "".to_owned(),
            tooltip_format: "Power profile: {name}".to_owned(),
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProfileEvent {
    ActiveProfile(Box<str>),
    PerformanceDegraded(Box<str>),
}

impl Module<Button> for PowerProfilesModule {
    type SendMessage = ProfileEvent;

    type ReceiveMessage = ();

    module_impl!("power_profiles");

    fn on_create(&mut self) {
        self.profiles.setup_defaults(default_profiles())
    }

    fn spawn_controller(
        &self,
        _info: &super::ModuleInfo,
        context: &super::WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: tokio::sync::mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()>
    where
        <Self as Module<Button>>::SendMessage: Clone,
    {
        const POLL_INTERVAL: Duration = Duration::from_millis(50);
        let tx = context.tx.clone();
        let client = context.try_client::<powerprofiles::Client>()?;

        spawn(async move {
            /// Switch to a `new_profile` updating `index`.
            fn switch_profile(
                profiles: &mut Vec<Box<str>>,
                new_profile: Box<str>,
                index: &mut usize,
            ) {
                if let Some(idx) = profiles.iter().position(|profile| *profile == new_profile) {
                    *index = idx;
                } else {
                    profiles.push(new_profile);
                    *index = profiles.len() - 1;
                }
            }

            let mut perf_reason = Box::from("");

            let mut profiles = match client.available_profiles().await {
                Ok(profiles) => profiles,
                Err(err) => {
                    error!("could not retrieve power profiles: {err}");
                    return;
                }
            };
            trace!("profiles: {profiles:?}");
            if profiles.is_empty() {
                error!("no profiles could be recovered");
                return;
            }
            let mut index = 0;

            tx.send_update(ProfileEvent::ActiveProfile(profiles[index].clone()))
                .await;

            let mut interval = time::interval(POLL_INTERVAL);
            loop {
                let event = tokio::select! {
                    event = rx.recv() => event,
                    _ = interval.tick() => None,
                };

                trace!("got event {event:?}");

                match client.get_new_active_profile(&profiles[index]).await {
                    Ok(profile) => {
                        if let Some(profile) = profile {
                            switch_profile(&mut profiles, profile, &mut index);
                            tx.send_update(ProfileEvent::ActiveProfile(profiles[index].clone()))
                                .await;
                        }
                    }
                    Err(err) => {
                        error!("could not retrieve an active profile: {err}");
                    }
                };

                match client
                    .get_new_performance_degraded_reason(&perf_reason)
                    .await
                {
                    Ok(reason) => {
                        if let Some(reason) = reason {
                            perf_reason = reason;
                            tx.send_update(ProfileEvent::PerformanceDegraded(perf_reason.clone()))
                                .await;
                        }
                    }
                    Err(err) => {
                        error!("could not retrieve a reason for performance degradation: {err}")
                    }
                }

                if event.is_some() {
                    let mut buf_idx = index + 1;
                    if buf_idx >= profiles.len() {
                        buf_idx = 0;
                    }

                    match client.set_profile(&profiles[buf_idx]).await {
                        Ok(_) => {
                            index = buf_idx;
                            tx.send_update(ProfileEvent::ActiveProfile(profiles[index].clone()))
                                .await;
                        }
                        Err(err) => {
                            error!("could not update power profile: {err:?}");
                        }
                    }
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: super::WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &super::ModuleInfo,
    ) -> Result<super::ModuleParts<Button>>
    where
        <Self as Module<Button>>::SendMessage: Clone,
    {
        let button_label = Label::builder()
            .use_markup(true)
            .justify(self.layout.justify.into())
            .css_classes(["label"])
            .build();

        let button_icon = IconLabel::new(
            "",
            default::IconSize::Small as i32,
            &context.ironbar.image_provider(),
        );
        button_icon.add_css_class("icon");

        let container = gtk::Box::new(self.layout.orientation(info), 5);
        container.add_css_class("contents");
        container.append(&*button_icon);
        container.append(&button_label);

        let button = Button::new();
        button.set_child(Some(&container));

        let tx = context.controller_tx.clone();
        button.connect_clicked(move |_| {
            tx.send_spawn(());
        });

        let rx = context.subscribe();

        let profile_button = button.clone();
        let mut manager = self.profiles.attach(
            &button,
            move |_, event: ProfileUpdateEvent<Box<str>, PowerProfilesProfile, ()>| {
                button_icon.set_label(Some(&event.profile.icon));
                button_label.set_label(&self.format.replace("{name}", &event.state));
                if !self.tooltip_format.is_empty() {
                    profile_button.set_tooltip_markup(Some(
                        &self.tooltip_format.replace("{name}", &event.state),
                    ));
                }
            },
        );

        let glib_button = button.clone();
        rx.recv_glib((), move |(), event| match event {
            ProfileEvent::ActiveProfile(profile) => manager.update(profile, ()),
            ProfileEvent::PerformanceDegraded(reason) => {
                const PERFORMANCE_CSS: &str = "performance_degraded";

                if reason.is_empty() {
                    glib_button.remove_css_class(PERFORMANCE_CSS);
                } else {
                    glib_button.add_css_class(PERFORMANCE_CSS);
                }
            }
        });

        Ok(ModuleParts::new(button, None))
    }
}
