use std::cmp::Ordering;
use glib::prelude::*;
use gtk::Widget;
use gtk::prelude::WidgetExt;
use serde::Deserialize;
use std::collections::HashMap;
use std::marker::PhantomData;

pub trait State: Default + Clone + PartialOrd {
    fn cmp(&self, other: &Self) -> Ordering;
}

impl State for i32 {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(self, other)
    }
}

impl State for f64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// A set of named value threshold-based profiles,
/// including a default fallback.
///
/// Modules using this struct should apply `#[serde(flatten)]`
/// to the field to avoid `profiles.profiles` syntax.
#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct Profiles<S, T>
where
    S: State,
    T: Default + Clone,
{
    profiles: HashMap<String, Profile<S, T>>,
    #[serde(flatten)]
    default: T,
}

/// An individual profile.
/// This represents the threshold at which it should be activated,
/// and the configuration associated with it.
#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct Profile<S, T>
where
    S: State,
    T: Default + Clone,
{
    /// The value threshold at which this profile should be activated.
    /// Values *less than or equal to* this will activate it
    /// (assuming the current value does not fall into a profile below this one).
    value: S,

    #[serde(flatten)]
    data: T,
}

impl<S, T> Profile<S, T>
where
    S: State,
    T: Default + Clone,
{
    pub fn new(value: S, data: T) -> Self {
        Self { value, data }
    }
}

impl<S, T> Profiles<S, T>
where
    S: State,
    T: Default + Clone,
{
    pub fn setup_defaults(&mut self, defaults: Profiles<S, T>) {
        for (name, profile) in defaults.profiles {
            self.profiles.entry(name).or_insert(profile);
        }
    }

    /// Attaches a 'primary' widget and callback update function to this profiles set,
    /// returning an update manager.
    ///
    /// # Example
    /// ```rs
    /// let btn = Button::new(Some("my button"));
    /// profiles.attach(&btn, |btn, event| {
    ///     println!(
    ///         "value: {} | profile: {:?} | data: {data:?}",
    ///         event.value, event.profile, event.data
    ///     );
    /// });
    /// ```
    pub fn attach<W, F, D>(&self, widget: &W, on_update: F) -> ProfilesManager<S, T, W, F, D>
    where
        W: IsA<Widget>,
        F: Fn(&W, ProfileUpdateEvent<S, T, D>),
    {
        ProfilesManager::new(self.clone(), widget.to_owned(), on_update)
    }
}

impl<S, T> From<HashMap<String, Profile<S, T>>> for Profiles<S, T>
where
    S: State,
    T: Default + Clone,
{
    fn from(profiles: HashMap<String, Profile<S, T>>) -> Self {
        Self {
            profiles,
            default: T::default(),
        }
    }
}

pub struct ProfilesManager<S, T, W, F, D>
where
    S: State,
    T: Default + Clone,
    W: IsA<Widget>,
    F: Fn(&W, ProfileUpdateEvent<S, T, D>),
{
    profiles: Profiles<S, T>,
    widget: W,
    on_update: F,

    profile_keys: Vec<String>,
    curr_profile: Option<String>,

    phantom_data: PhantomData<D>,
}

pub struct ProfileUpdateEvent<'a, S, T, D> {
    pub state: S,
    pub profile: &'a T,
    pub data: D,
}

impl<S, T, W, F, D> ProfilesManager<S, T, W, F, D>
where
    S: State,
    T: Default + Clone,
    W: IsA<Widget>,
    F: Fn(&W, ProfileUpdateEvent<S, T, D>),
{
    fn new(profiles: Profiles<S, T>, widget: W, on_update: F) -> Self {
        let p_map = &profiles.profiles;
        let mut profile_keys = p_map.keys().map(ToOwned::to_owned).collect::<Vec<_>>();

        profile_keys.sort_by(|a, b| p_map[a].value.cmp(&p_map[b].value));

        Self {
            profiles,
            widget,
            profile_keys,
            on_update,
            curr_profile: None,
            phantom_data: PhantomData,
        }
    }

    /// Sends an update to the manager,
    /// with the new input value and any associated data.
    ///
    /// The new profile is determined from the value,
    /// and passed with the value and data to the `on_change` callback.
    ///
    /// The attached primary widget is additionally updated
    /// to include the active profile name as a classname.
    ///
    /// Note that the `on_update` callback runs every time this is called,
    /// regardless of whether the profile has changed.
    pub fn update(&mut self, value: S, data: D) {
        let new_profile_name = self.profile_keys.iter().find(|&name| {
            let profile = &self.profiles.profiles[name];
            profile.value >= value
        });

        let profile = if let Some(name) = new_profile_name {
            &self.profiles.profiles[name].data
        } else {
            &self.profiles.default
        };

        if new_profile_name != self.curr_profile.as_ref() {
            self.update_classes(new_profile_name);
            self.curr_profile = new_profile_name.cloned();
        }

        let update_data = ProfileUpdateEvent {
            state: value,
            profile,
            data,
        };

        (self.on_update)(&self.widget, update_data);
    }

    fn update_classes(&self, new_profile: Option<&String>) {
        for profile in &self.profile_keys {
            let class = format!("profile-{profile}");
            if Some(profile) == new_profile {
                self.widget.add_css_class(&class);
            } else {
                self.widget.remove_css_class(&class);
            }
        }
    }
}

#[macro_export]
macro_rules! profiles {
    ($($name:literal:$threshold:literal => $value:expr),+) => {{
        let mut map = std::collections::HashMap::new();
        $(map.insert($name.to_string(), $crate::config::Profile::new($threshold, $value));)+
        Profiles::from(map)
    }};
}
