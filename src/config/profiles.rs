use glib::prelude::*;
use gtk::Widget;
use gtk::prelude::WidgetExt;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::marker::PhantomData;

/// A profile state/matcher.
/// This is used as the "threshold" activation,
/// and can be implemented on any type.
/// General advice is to keep the state type as simple as possible.
/// State objects must also be *orderable* in a logical manner.
///
/// Complex state objects must manually implement the `PartialOrd` and `State` traits.
/// Deriving `PartialOrd` will likely produce an incorrect sorter.
///
/// The PartialOrd implementation must obey the following rules:
///
/// - The primary field must be sorted first (e.g. battery percentage)
/// - For optional fields, a `Some` variant is *less* than a `None`.
///   This is because internally the first matching profile is used.
///
/// ```rs
/// impl PartialOrd for ProfileState {
///     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
///         if self.percent == other.percent {
///             // prefer profiles with a specified `charging` value.
///             match (self.charging, other.charging) {
///                 (Some(_), Some(_)) | (None, None) => Some(Ordering::Equal),
///                 (None, Some(_)) => Some(Ordering::Greater),
///                 (Some(_), None) => Some(Ordering::Less),
///             }
///         } else {
///             self.percent.partial_cmp(&other.percent)
///         }
///     }
/// }
/// ```
///
/// Implementations are provided for all number types.
pub trait State: Default + Clone + PartialOrd {
    /// Returns `true` if this state definition matches the provided value.
    ///
    /// The default implementation provides a `value <= self` case,
    /// which is sufficient for any numeric implementation.
    ///
    /// More complex cases, such as those handling `Option<T>` fields
    /// will need to override this to account for the logic.
    ///
    /// # Example
    ///
    /// ```rs
    /// fn matches(&self, value: &Self) -> bool {
    ///     match self.charging {
    ///         Some(charging) => {
    ///             charging == value.charging.expect("should exist")
    ///                 && value.percent <= self.percent
    ///         }
    ///         None => value.percent <= self.percent,
    ///     }
    /// }
    /// ```
    fn matches(&self, value: &Self) -> bool {
        value <= self
    }
}
impl State for u8 {}
impl State for u16 {}
impl State for u32 {}
impl State for u64 {}
impl State for u128 {}
impl State for i8 {}
impl State for i16 {}
impl State for i32 {}
impl State for i64 {}
impl State for i128 {}
impl State for usize {}
impl State for isize {}
impl State for f32 {}
impl State for f64 {}

/// A set of named value threshold-based profiles,
/// including a default fallback.
///
/// Modules using this struct should apply `#[serde(flatten)]`
/// to the field to avoid `profiles.profiles` syntax.
///
/// `S` = State, `T` = Configuration data.
#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct Profiles<S, T>
where
    S: State,
    T: Default + Clone,
{
    /// A map of named profiles against profile entries.
    /// Entries can be provided in two formats.
    ///
    /// The profile entry object differs per-module-
    /// check module property documentation to see if they are supported.
    ///
    /// See [profiles] for more information.
    profiles: HashMap<String, ProfileEntry<S, T>>,

    /// The default profile.
    #[serde(flatten)]
    default: T,
}

/// Represents a single entry in the `profiles` map.
///
/// NOTE: Enum variant order matters here.
/// Serde will attempt to match in order,
/// which will incorrectly resolve all objects as `Simple`
/// due to `Default` constraint.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ProfileEntry<S, T>
where
    S: State,
    T: Default + Clone,
{
    /// A full profile object with configuration attached.
    ///
    /// # Example
    ///
    /// ```corn
    /// { profiles.example.format = "hello" }
    /// ```
    Full(Profile<S, T>),
    /// A state-matcher only to add styling classes
    /// without any configuration attached.
    ///
    /// # Example
    ///
    /// ```corn
    /// { profiles.example = 42 }
    /// ```
    Simple(S),
}

impl<S, T> ProfileEntry<S, T>
where
    S: State,
    T: Default + Clone,
{
    fn when(&self) -> &S {
        match self {
            ProfileEntry::Full(profile) => &profile.when,
            ProfileEntry::Simple(when) => when,
        }
    }
}

impl<S, T> From<Profile<S, T>> for ProfileEntry<S, T>
where
    S: State,
    T: Default + Clone,
{
    fn from(value: Profile<S, T>) -> Self {
        Self::Full(value)
    }
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
    /// The 'state' value threshold at which this profile should be activated.
    /// Values *less than or equal to* this will activate it
    /// (assuming the current value does not fall into a profile below this one).
    when: S,

    /// The configuration data attached to this profile.
    #[serde(flatten)]
    data: T,
}

impl<S, T> Profile<S, T>
where
    S: State,
    T: Default + Clone,
{
    /// Creates a new profile for this matcher `when`,
    /// and the associated profile data.
    pub fn new(when: S, data: T) -> Self {
        Self { when, data }
    }
}

impl<S, T> Profiles<S, T>
where
    S: State,
    T: Default + Clone,
{
    /// Merges a default profile set into this one,
    /// adding keys from the default if they do not already exist.
    ///
    /// Existing profiles are not overridden.
    ///
    /// # Example
    ///
    /// ```rs
    /// impl Module<Button> for VolumeModule {
    ///   fn on_create(&mut self) {
    ///       self.profiles.setup_defaults(profiles!(
    ///         "low":33 => VolumeProfile::for_volume_icon("󰕿"),
    ///         "medium":67 => VolumeProfile::for_volume_icon("󰖀")
    ///     ));
    ///   }
    /// }
    /// ```
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
            profiles: profiles
                .into_iter()
                .map(|(name, profile)| (name, profile.into()))
                .collect(),
            default: T::default(),
        }
    }
}

/// A manager for a set of profiles,
/// used to determine which profile to load and
/// send updates to the attached callback.
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

/// An event sent by a `ProfilesManager` when `update` is called.
pub struct ProfileUpdateEvent<'a, S, T, D> {
    /// The state provided in the update.
    /// Note this is not the state matcher of the profile itself.
    pub state: S,
    /// The profile configuration data.
    pub profile: &'a T,
    /// Any additional non-profile data passed in the update.
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

        profile_keys.sort_by(|a, b| {
            p_map[a]
                .when()
                .partial_cmp(p_map[b].when())
                .unwrap_or(Ordering::Equal)
        });

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
    /// with the new input value
    /// and any associated non-profile data.
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
            profile.when().matches(&value)
        });

        let profile = new_profile_name
            .map(|name| &self.profiles.profiles[name])
            .and_then(|profile| match profile {
                ProfileEntry::Full(profile) => Some(&profile.data),
                ProfileEntry::Simple(_) => None,
            })
            .unwrap_or(&self.profiles.default);

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
    ($($name:literal:$threshold:expr => $value:expr),+) => {{
        let mut map = std::collections::HashMap::new();
        $(map.insert($name.to_string(), $crate::config::Profile::new($threshold, $value));)+
        Profiles::from(map)
    }};
}
