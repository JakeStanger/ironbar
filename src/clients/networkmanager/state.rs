use zbus::zvariant::ObjectPath;

use super::dbus::{DeviceState, DeviceType};

#[derive(Clone, Debug)]
pub struct Device {
    pub path: ObjectPath<'static>,
    /// Interface             readable   s
    pub interface: String,

    /// State                 readable   u
    ///
    /// The current state of the device.
    pub state: DeviceState,

    /// Ip4Config             readable   o
    ///
    /// Object path of the Ip4Config object describing the configuration of the device. Only valid
    /// when the device is in the NM_DEVICE_STATE_ACTIVATED state.
    pub ip4_config: Option<Ip4Config>,

    /// DeviceType            readable   u
    ///
    /// The general type of the network device; ie Ethernet, WiFi, etc.
    pub device_type: DeviceType,
    /// Device data specific to the device type.
    pub device_type_data: DeviceTypeData,
    //
    // # Unmapped properties:
    // Udi                   readable   s
    // IpInterface           readable   s
    // Driver                readable   s
    // DriverVersion         readable   s
    // FirmwareVersion       readable   s
    // Capabilities          readable   u
    // Ip4Address            readable   u
    // StateReason           readable   (uu)
    // ActiveConnection      readable   o
    // Dhcp4Config           readable   o
    // Ip6Config             readable   o
    // Dhcp6Config           readable   o
    // Managed               readwrite  b
    // Autoconnect           readwrite  b
    // FirmwareMissing       readable   b
    // NmPluginMissing       readable   b
    // AvailableConnections  readable   ao
    // PhysicalPortId        readable   s
    // Mtu                   readable   u
    // Metered               readable   u
    // LldpNeighbors         readable   aa{sv}
    // Real                  readable   b
}

#[derive(Clone, Debug)]
pub struct Ip4Config {
    pub path: ObjectPath<'static>,
    /// AddressData  readable   aa{sv}
    ///
    /// Array of IP address data objects. All addresses will include "address" (an IP address
    /// string), and "prefix" (a uint). Some addresses may include additional attributes.
    pub address_data: Vec<AddressData>,
    //
    // # Unmapped properties:
    // Addresses    readable   aau
    // Gateway      readable   s
    // Routes       readable   aau
    // RouteData    readable   aa{sv}
    // Nameservers  readable   au
    // Domains      readable   as
    // Searches     readable   as
    // DnsOptions   readable   as
    // DnsPriority  readable   i
    // WinsServers  readable   au
}

#[derive(Clone, Debug)]
pub struct AddressData {
    pub address: String,
    pub prefix: u32,
}

/// The sub-interface data for the device, e.g. wifi, etc.
#[derive(Clone, Debug)]
pub enum DeviceTypeData {
    /// The device does not have a specific type, or it is unimplemented.
    None,
    Wireless(DeviceWireless),
}

#[derive(Clone, Debug)]
pub struct DeviceWireless {
    /// ActiveAccessPoint     readable   o
    ///
    /// Object path of the access point currently used by the wireless device.
    pub active_access_point: Option<AccessPoint>,
    //
    // # Unmapped properties:
    // HwAddress             readable   s
    // PermHwAddress         readable   s
    // Mode                  readable   u
    // Bitrate               readable   u
    // AccessPoints          readable   ao
    // WirelessCapabilities  readable   u
}

#[derive(Clone, Debug)]
pub struct AccessPoint {
    pub path: ObjectPath<'static>,
    /// Ssid        readable   ay
    ///
    /// The Service Set Identifier identifying the access point.
    pub ssid: Vec<u8>,

    /// Strength    readable   y
    ///
    /// The current signal quality of the access point, in percent.
    pub strength: u8,
    //
    // # Unmapped properties:
    // Frequency   readable   u
    // HwAddress   readable   s
    // Mode        readable   u
    // MaxBitrate  readable   u
    // Flags       readable   u
    // WpaFlags    readable   u
    // RsnFlags    readable   u
    // LastSeen    readable   i
}
