Ironbar is a single monolithic Rust project, with all code inside the standard `src` folder.

The three most important code modules to be aware of when expanding Ironbar are as follows: 

- `clients` - Contains code for interacting with external services, such as Wayland, D-Bus, etc.
- `ipc` - Contains commands and responses, and implementation code for the IPC server. Commands/responses also map to the CLI automatically.
- `modules` - Contains module-specific code.

Each of the other modules are considered part of the core functionality:

- `config` - Top-level and bar-level configuration definitions, and common config code.
- `dynamic_value` - Implementations for dynamic values and their bindings to scripts/ironvars.
- `image` - Image parsing and loading code
- Top-level core code

This guide covers the client, ipc, and module creation process.

## Creating a new client

Any new module is likely going to need to talk to some external service. 
A client module should be created to house the code for this.

Each client should be a singleton (or map of singletons where applicable), so that it can be shared by the entire application.
It must be obtainable from the `Clients` struct. 

Additionally, the `clients::register_client` or `clients::register_fallible_client` macros should be used
to derive the `MethodContext::provide` method, 
allowing for `context.get_client` and `context.try_get_client` to be called by widgets.

The first implementation step is to hold the instance on the `Clients` struct,
and add a method to be able to get it. The method body will vary depending on how the client is initialized.

The below example shows how the volume client is implemented:

```rust
#[cfg(feature = "volume")]
pub mod volume;

// ...

struct Clients {
  // ...
  
  #[cfg(feature = "volume")]
  volume: Option<Arc<volume::Client>>

  // ...
}

impl Clients {
  // ...

  #[cfg(feature = "volume")]
  pub fn volume(&mut self) -> Arc<volume::Client> {
    self.volume
            .get_or_insert_with(volume::create_client)
            .clone()
  }
  
  // ...
}
```

The actual `Client` implementation can vary greatly depending on the underlying service
and intended use-case.

Some general guidelines:

- Connections to the underlying service should be minimised. 
  Ideally, there should be one that exists for the application lifetime.
  In some cases, more than one may be required due to `Send`/`Sync` limitations.
- The underlying service should be obfuscated as much as reasonably possible. 
  Effectively, any service consumers should not need to know about which OS component is being used. 
- The client needs to provide a way of getting the current state of the service.
  This is used to populate a module instance when it is created. 
  As clients exist for the application lifetime, and modules can be created/destroyed on config changes, this can be any time.
  There are two general patterns to achieve this:
  - The client provides a method to get the current state. This is preferable where possible.
  - The `subscribe` method sends an `Init` event *immediately* after calling `tx.subscribe()`, containing all state.
- The client needs to provide a way of subscribing to events from the service.
  This is normally achieved using a Tokio `broadcast` channel, and a `subscribe` method to create a new subscription to the `tx`.
  Each module can then call `client.subscribe()` in its controller to start listening to events.
- Requests back to the underlying service should be exposed as methods in the client.

As a simple example, the full `notifications` client for talking to SwayNC is broken down below.

This starts with the struct, which holds the proxy for the application lifetime.
It also holds a `broadcast` channel instance. The `tx` is held to allow for subscriptions, 
and the `rx` is held to ensure the channel is not immediately dropped.

> [!TIP]
> It is also possible to use `std::mem::forget(rx)` to clean up `rx` without calling its `Drop` implementation.
> This leaves the channel open.

```rust
#[derive(Debug)]
pub struct Client {
    proxy: SwayNcProxy<'static>,
    tx: broadcast::Sender<Event>,
    _rx: broadcast::Receiver<Event>,
}
```

The implementation provides an `async` constructor. 
This both creates and returns the client instance and sets up a task to listen to events from D-Bus.

```rust
impl Client {
    pub async fn new() -> Result<Self> {
        let dbus = Box::pin(zbus::Connection::session()).await?;

        let proxy = SwayNcProxy::new(&dbus).await?;
        let (tx, rx) = broadcast::channel(8);

        let mut stream = proxy.receive_subscribe_v2().await?;

        {
            let tx = tx.clone();

            spawn(async move {
                while let Some(ev) = stream.next().await {
                    let ev = ev
                        .message()
                        .body()
                        .deserialize::<Event>()
                        .expect("to deserialize");
                    debug!("Received event: {ev:?}");
                    tx.send_expect(ev);
                }
            });
        }

        Ok(Self { proxy, tx, _rx: rx })
    }

    // ...
}
```

With the skeleton client in place and receiving events from D-Bus,
we can write the methods needed for consumers to be able to interact with the service.

The `subscribe` method returns a broadcast receiver, allowing modules to listen to events.
Logic here should be kept to a minimum, and absolutely no async work should occur 
between calling `subscribe` and returning the resultant receiver.

Next is the `state` method, which returns the current service state. This is used for module initialisation.
In this case, we are able to get the full state in one call from D-Bus and convert the struct.
More complex cases will likely need multiple calls, or will need to cache data.

The last method, `toggle_visibility`, sends a request back to the service.
This method is called by the module controller when the user clicks the module toggle button.

```rust
impl Client {
    // ...
  
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    pub async fn state(&self) -> Result<Event> {
        debug!("Getting subscribe data (current state)");
        match self.proxy.get_subscribe_data().await {
            Ok(data) => Ok(data.into()),
            Err(err) => Err(Report::new(err)),
        }
    }

    pub async fn toggle_visibility(&self) {
        debug!("Toggling visibility");
        if let Err(err) = self.proxy.toggle_visibility().await {
            error!("{err:?}");
        }
    }
}
```

### Implementing Ironvar support

Where possible, client information should be exposed as read-only ironvars.
This allows querying of client information over IPC, 
and will eventually allow for client information to be dynamically read in custom modules.

Each client registers its own top-level namespace, which is a type implementing `ironvar::Namespace`. 
Where possible this should be the Client type itself.

The `VariableManager::register_namespace` method is used to perform the registration.
Typically, this is performed when the client is created, as with the sysinfo example below:

```rust
impl Clients {
    #[cfg(feature = "sys_info")]
    pub fn sys_info(&mut self) -> Arc<sysinfo::Client> {
        self.sys_info
            .get_or_insert_with(|| {
                let client = Arc::new(sysinfo::Client::new());

                #[cfg(feature = "ipc")]
                Ironbar::variable_manager().register_namespace("sysinfo", client.clone()); // <-- register ironvar namespace

                client
            })
            .clone()
  }
}
```

The `Namespace` implementation requires four methods be implemented:

- `get` - Takes a key, and returns the (stringified) value associated with it, if one exists.
- `list` - Returns a list of all keys in the namespace. This does not include child namespaces.
- `get_namespace` - Takes a key, and returns the child namespace associated with it, if one exists.
- `namespaces` - Returns a list of all child namespaces.

Child namespaces can be used to logically group keys.
For example, the `sysinfo` client registers a child namespace called `cpu_frequency` to hold CPU frequency information.
This namespace then contains child values for each CPU core, and aggregate values.

The implementation can be as static or dynamic as necesary, depending on the use-case.
Performing I/O to fetch values is generally considered okay, since values are generally queried on an ad-hoc basis.

## Creating a new module

Each module has its own file in the `modules` folder, or its own subfolder if complex enough.

### Config

The module consists of a struct that represents its configuration.
This config is "digested" when the bar loads to form the specific module instance.

At a minimum, each module struct must derive all of `Debug`, `serde::Deserialize`, and `Clone`. 
When the `extras` feature is enabled, it must also derive `schemars::JsonSchema`.

An extract of the Clock module is shown below as an example:

```rust
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ClockModule {
    /// The format string to use for the date/time shown on the bar.
    /// Pango markup is supported.
    ///
    /// Detail on available tokens can be found here:
    /// <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>
    ///
    /// **Default**: `%d/%m/%Y %H:%M`
    format: String,
    
    // ...
  
    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

// by using `#[serde(default)]` at the struct level,
// default values are automatically merged in where not specified.
impl Default for ClockModule {
    fn default() -> Self {
        Self {
            format: "%d/%m/%Y %H:%M".to_string(),
            // ...
        }
    }
}
```

Every module must include a `common` field, configured exactly as above. 
The value of this is taken at module creation time, so is not accessible from the module implementation.

> [!TIP]
> Future versions of Ironbar will generate module documentation directly from the config structs.
> Documentation should therefore be included on each config field when writing the module.

#### Profiles

Profiles can be added to a module's configuration to allow users to apply different configurations depending on a module's state.
This is normally based off a simple numerical metric, such as the volume or battery percentage. 
Additional values can be attached to state where required.

Within a configuration struct you'd place a `profiles` field of type `Profiles<S, T>`. 

- `S` represents the state/matcher type, which must implement the `State` trait.
- `T` represents the configuration contained within a profile. This is any deserializable type.

You **must** include `#[serde(flatten)]` on the field to avoid a double `profiles.profiles` syntax 
and ensure the default is merged at the top-level correctly.

```rust
struct VolumeModule {
  #[serde(flatten)]
  pub(super) profiles: Profiles<f64, VolumeProfile>,
}
```

```rust
struct BatteryModule {
    #[serde(flatten)]
    profiles: Profiles<ProfileState, BatteryProfile>,
}
```

Profile state should aim to be as simple as possible, and only include what is absolutely necessary.
The profile configuration can be as complex as desired, but again should only include options that make sense to include.
For example, changing the click behaviour based on module state would likely be undesirable. 

Each module should provide a default implementation for its profile type, 
and a set of appropriate default profiles where appropriate.

```rs
#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct VolumeProfile {
    pub(super) icons: Icons,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            volume: "󰕾".to_string(),
            muted: "󰝟".to_string(),
        }
    }
}
```

### Compound state

Implementations for numeric types are provided, 
however complex state objects must manually implement the `PartialOrd` and `State` traits. 
Deriving `PartialOrd` will likely produce an incorrect sorter.

The `PartialOrd` implementation must obey the following rules:

- The primary field must be sorted first (eg battery percentage)
- For optional fields, a `Some` variant is *less* than a `None`. 
  This is because internally the first matching profile is used.

```rs
impl PartialOrd for ProfileState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.percent == other.percent {
            match (self.charging, other.charging) {
                (Some(_), Some(_)) | (None, None) => Some(Ordering::Equal),
                (None, Some(_)) => Some(Ordering::Greater),
                (Some(_), None) => Some(Ordering::Less),
            }
        } else {
            self.percent.partial_cmp(&other.percent)
        }
    }
}
```

The state implementation requires a single method, 
which checks whether an input state matches the profile matcher. 
Again optional fields need to be considered here - 
if omitted in `&self` (the config value), matching should ignore this field.

```rs
impl State for ProfileState {
    fn matches(&self, value: &Self) -> bool {
        match self.charging {
            Some(charging) => {
                charging == value.charging.expect("value should exist")
                    && value.percent <= self.percent
            }
            None => value.percent <= self.percent,
        }
    }
}
```


### Implementation

The module behaviour itself is implemented using the `crate::modules::Module` trait. 
This takes a generic argument which determines the GTK widget type returned by this module. 
In most cases, this is likely to be a `Box`, `Label`, or `Button`, although all widget types can be used in theory. 

The implementation for this trait can be thought of as being split into four parts:

- **Header**: Contains metadata information.
- **Controller**: Contains logic for bridging between the client and UI code.
- **Widget**: Bar UI code.
- **Popup**: Popup UI code.

The controller, widget, and popup each take a `ModuleInfo` and `WidgetContext` struct. 
The info struct contains information relevant to the module's location and some general global options
such as the icon theme. The context holds instances of the bar, popup, and communication channels.

#### Header

Every module needs the following types:

- `SendMessage` - the type of data sent **FROM** the controller **TO** the UI. This usually represents an update from the client.
- `ReceiveMessage` - the type of data sent **FROM** the UI **TO** the controller. This usually represents a user interaction.

Each module also requires a `name` method used to determine its `type` config option, 
and `take_common` method used to grab the common options. 
These can be automatically generated using the `crate::module_impl` macro.

```rust
impl Module<Button> for ClockModule {
    type SendMessage = DateTime<Local>;
    type ReceiveMessage = (); // clock ui does not send events to its controller.

    module_impl!("clock");
    
    // ...
}
```

Modules may also provide an implementation for `on_create`.
This runs immediately after the module is created, and before the controller.
It takes `&mut self`, allowing for minor tweaks to the configuration where appropriate.
This may be used for example to work around Serde limitations.

In general, the body of `on_create` should be as minimal as possible.

#### Controller

The intention of the controller is to provide a separation of concerns 
and ensure that the UI code is responsible for the UI and nothing else.

A typical controller will hold an instance of the client, perform initialisation,
listen to client events, and send client requests based on user actions.

Generally this is achieved by spawning one or more Tokio tasks, 
allowing for asynchronous communication separate from the UI thread.

> [!TIP]
> Tokio tasks should be spawned using `crate::spawn` and not `tokio::spawn`.
> This is because the UI thread is outside the Tokio runtime 
> and will panic if you try to spawn a task on it.

The controller is spawned just before the UI is created and takes reference of the config struct.

You can use `context.tx` to send a `SendMessage` to the UI, and `rx` to receive `ReceiveMessage` from the UI.

Client instances can be obtained using `context.client::<TClient>()` or `context.try_client::<TClient>()`.
This will return a singleton client shared by the entire application, lazy-loaded as required.

The controller from the `notifications` module is shown below as a simple example. This spawns two tasks:

- The first is responsible for communicating with the client. 
  It gets the initial state and sends it to the UI, then listens to incoming events to forward to the UI.
- The second listens to events from the UI and calls the relevant client method.

```rust
impl Module<Overlay> for NotificationsModule {
    // ...
    
    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> color_eyre::Result<()>
    where
        <Self as Module<Overlay>>::SendMessage: Clone,
    {
        let client = context.try_client::<swaync::Client>()?;

        {
            let client = client.clone();
            let mut rx = client.subscribe();
            let tx = context.tx.clone();

            spawn(async move {
                // init
                // some clients provide a mechanism for getting the current state.
                // others may send an `Init` event when subscribing.
                let initial_state = client.state().await;

                match initial_state {
                    Ok(ev) => tx.send_update(ev).await,
                    Err(err) => error!("{err:?}"),
                };
                
                // listen to client events
                // in this case we forward them with no further processing -
                // a more complex controller may need to process incoming events here.
                while let Ok(ev) = rx.recv().await {
                    tx.send_update(ev).await;
                }
            });
        }

        // listen to UI events
        spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    UiEvent::ToggleVisibility => client.toggle_visibility().await,
                }
            }
        });

        Ok(())
    }
    
    // ...
}
```

#### Widget

The `into_widget` method is responsible for consuming the config struct,
and returning a fully functional GTK widget ready to be added to the bar.

This normally involves creating a top-level widget, adding any child widgets,
and connecting any signals to talk back to the controller.

The widget receives a transmitter to the same channel as the controller. 
The primary use of this from the UI side is to send events requesting the popup is opened/closed. 
It may also serve as a useful escape hatch in situations where the controller cannot be used 
(when data is constrained to the Glib runtime thread).

```rust
fn into_widget(
    self,
    context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
    info: &ModuleInfo,
) -> Result<ModuleParts<Button>> {
    // ...
    
    let tx = context.tx.clone();
    button.connect_clicked(move |button| {
        try_send!(tx, ModuleUpdateEvent::TogglePopup(button.popup_id()));
    });

    // ...
}
```

To receive `SendMessage` events from the controller, the `recv_glib` extension method can be used.
This provides a mechanism for receiving events from the channel asynchronously, while ensuring it remains on the UI thread.

The first argument of the method allows for a "dependency list" to be provided, which is internally cloned and passed to the closure. 
This avoids the need to `.clone()` everything moved into the closure. 
You can leave this empty or pass up to 12 parameters as needed.

```rust
fn into_widget(
    self,
    context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
    info: &ModuleInfo,
) -> Result<ModuleParts<Button>> {
    // ...
    
    let rx = context.subscribe();
    rx.recv_glib((&label), move |(label), date| {
        let date_string = format!("{}", date.format_localized(&format, locale));
        label.set_label(&date_string);
    });
    
    // ...
}
```

#### Popup

The popup implementation is very similar to the widget, with a few key differences:

- The implementation is optional. the trait includes a default implementation which returns `None`. 
- The popup will always return a `Box` widget.
- The method must be called manually at the end of your `into_widget` implementation.

> [!TIP]
> You must also call `into_popup_parts` to create a `ModulePopupParts` struct.
> This takes a list of button widgets which are capable of opening the popup.
> 
> You can also use `into_popup_parts_owned` or `into_popup_parts_with_finder` depending on the use-case.

```rust
fn into_widget(
    self,
    context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
    info: &ModuleInfo,
) -> Result<ModuleParts<Button>> {
    // ...
    
    let popup = self
        .into_popup(context, info)
        .into_popup_parts(vec![&button]);

    Ok(ModuleParts::new(button, popup))
}
```

The actual implementation would then look something like the below:

```rust
 fn into_popup(
        self,
        tx: mpsc::Sender<Self::ReceiveMessage>,
        rx: broadcast::Receiver<Self::SendMessage>,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Option<gtk::Box> {
    let container = gtk::Box::new(Orientation::Vertical, 0);
    
    // ...
    
    Some(container)
}
```

## Creating a new IPC command

Commands are initially defined in the `crate::ipc::Command` module. 
This is an enum, containing child enums and structs which represents each possible command.

The struct is shared by the CLI implementation, so necessary `clap` traits must be derived as appropriate.
This automatically handles the client-side implementation.

On the server side, the command needs to be handled inside the `crate::ipc::server` module, 
in the `Ipc::handle_command` method.

This takes the top-level subcommand and forwards it onto the child module.
Each child module has its own `handle_command` method.
This method simply maps matches the passed command, 
performs the required operation, and then returns an `ipc::Response`.

Taken below is an extract from the Ironvars IPC module:

```rust

pub fn handle_command(command: IronvarCommand) -> Response {
    match command {
        IronvarCommand::Set { key, value } => {
            let variable_manager = Ironbar::variable_manager();
            match variable_manager.set(&key, value) {
                Ok(()) => Response::Ok,
                Err(err) => Response::error(&format!("{err}")),
            }
        }
      // ...
    }
}
```