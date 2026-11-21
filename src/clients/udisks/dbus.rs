use zbus::proxy;

#[proxy(
    default_service = "org.freedesktop.UDisks2",
    interface = "org.freedesktop.UDisks2.Drive",
    assume_defaults = true
)]
pub trait Drive {
    /// Eject method
    fn eject(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// PowerOff method
    fn power_off(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// SetConfiguration method
    fn set_configuration(
        &self,
        value: std::collections::HashMap<&str, &zbus::Variant<'_>>,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// CanPowerOff property
    #[zbus(property)]
    fn can_power_off(&self) -> zbus::Result<bool>;

    /// Configuration property
    #[zbus(property)]
    fn configuration(&self) -> zbus::Result<std::collections::HashMap<String, zbus::OwnedVariant>>;

    /// ConnectionBus property
    #[zbus(property)]
    fn connection_bus(&self) -> zbus::Result<String>;

    /// Ejectable property
    #[zbus(property)]
    fn ejectable(&self) -> zbus::Result<bool>;

    /// Id property
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    /// Media property
    #[zbus(property)]
    fn media(&self) -> zbus::Result<String>;

    /// MediaAvailable property
    #[zbus(property)]
    fn media_available(&self) -> zbus::Result<bool>;

    /// MediaChangeDetected property
    #[zbus(property)]
    fn media_change_detected(&self) -> zbus::Result<bool>;

    /// MediaCompatibility property
    #[zbus(property)]
    fn media_compatibility(&self) -> zbus::Result<Vec<String>>;

    /// MediaRemovable property
    #[zbus(property)]
    fn media_removable(&self) -> zbus::Result<bool>;

    /// Model property
    #[zbus(property)]
    fn model(&self) -> zbus::Result<String>;

    /// Optical property
    #[zbus(property)]
    fn optical(&self) -> zbus::Result<bool>;

    /// OpticalBlank property
    #[zbus(property)]
    fn optical_blank(&self) -> zbus::Result<bool>;

    /// OpticalNumAudioTracks property
    #[zbus(property)]
    fn optical_num_audio_tracks(&self) -> zbus::Result<u32>;

    /// OpticalNumDataTracks property
    #[zbus(property)]
    fn optical_num_data_tracks(&self) -> zbus::Result<u32>;

    /// OpticalNumSessions property
    #[zbus(property)]
    fn optical_num_sessions(&self) -> zbus::Result<u32>;

    /// OpticalNumTracks property
    #[zbus(property)]
    fn optical_num_tracks(&self) -> zbus::Result<u32>;

    /// Removable property
    #[zbus(property)]
    fn removable(&self) -> zbus::Result<bool>;

    /// Revision property
    #[zbus(property)]
    fn revision(&self) -> zbus::Result<String>;

    /// RotationRate property
    #[zbus(property)]
    fn rotation_rate(&self) -> zbus::Result<i32>;

    /// Seat property
    #[zbus(property)]
    fn seat(&self) -> zbus::Result<String>;

    /// Serial property
    #[zbus(property)]
    fn serial(&self) -> zbus::Result<String>;

    /// SiblingId property
    #[zbus(property)]
    fn sibling_id(&self) -> zbus::Result<String>;

    /// Size property
    #[zbus(property)]
    fn size(&self) -> zbus::Result<u64>;

    /// SortKey property
    #[zbus(property)]
    fn sort_key(&self) -> zbus::Result<String>;

    /// TimeDetected property
    #[zbus(property)]
    fn time_detected(&self) -> zbus::Result<u64>;

    /// TimeMediaDetected property
    #[zbus(property)]
    fn time_media_detected(&self) -> zbus::Result<u64>;

    /// Vendor property
    #[zbus(property)]
    fn vendor(&self) -> zbus::Result<String>;

    /// WWN property
    #[zbus(property, name = "WWN")]
    fn wwn(&self) -> zbus::Result<String>;
}

#[proxy(
    interface = "org.freedesktop.UDisks2.Filesystem",
    default_service = "org.freedesktop.UDisks2"
)]
pub trait Filesystem {
    /// Check method
    fn check(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<bool>;

    /// Mount method
    fn mount(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<String>;

    /// Repair method
    fn repair(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<bool>;

    /// Resize method
    fn resize(
        &self,
        size: u64,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// SetLabel method
    fn set_label(
        &self,
        label: &str,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// SetUUID method
    #[zbus(name = "SetUUID")]
    fn set_uuid(
        &self,
        uuid: &str,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// TakeOwnership method
    fn take_ownership(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// Unmount method
    fn unmount(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// MountPoints property
    #[zbus(property)]
    fn mount_points(&self) -> zbus::Result<Vec<Vec<u8>>>;

    /// Size property
    #[zbus(property)]
    fn size(&self) -> zbus::Result<u64>;
}

#[proxy(
    interface = "org.freedesktop.UDisks2.Block",
    default_service = "org.freedesktop.UDisks2"
)]
pub trait Block {
    /// AddConfigurationItem method
    fn add_configuration_item(
        &self,
        item: &(&str, std::collections::HashMap<&str, &zbus::Variant<'_>>),
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// Format method
    fn format(
        &self,
        type_: &str,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// GetSecretConfiguration method
    fn get_secret_configuration(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<
        Vec<(
            String,
            std::collections::HashMap<String, zbus::OwnedVariant>,
        )>,
    >;

    /// OpenDevice method
    fn open_device(
        &self,
        mode: &str,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<zbus::OwnedFd>;

    /// OpenForBackup method
    fn open_for_backup(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<zbus::OwnedFd>;

    /// OpenForBenchmark method
    fn open_for_benchmark(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<zbus::OwnedFd>;

    /// OpenForRestore method
    fn open_for_restore(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<zbus::OwnedFd>;

    /// RemoveConfigurationItem method
    fn remove_configuration_item(
        &self,
        item: &(&str, std::collections::HashMap<&str, &zbus::Variant<'_>>),
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// Rescan method
    fn rescan(
        &self,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// RestoreEncryptedHeader method
    fn restore_encrypted_header(
        &self,
        backup_file: &str,
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// UpdateConfigurationItem method
    fn update_configuration_item(
        &self,
        old_item: &(&str, std::collections::HashMap<&str, &zbus::Variant<'_>>),
        new_item: &(&str, std::collections::HashMap<&str, &zbus::Variant<'_>>),
        options: std::collections::HashMap<&str, &zbus::Variant<'_>>,
    ) -> zbus::Result<()>;

    /// Configuration property
    #[zbus(property)]
    fn configuration(
        &self,
    ) -> zbus::Result<
        Vec<(
            String,
            std::collections::HashMap<String, zbus::OwnedVariant>,
        )>,
    >;

    /// CryptoBackingDevice property
    #[zbus(property)]
    fn crypto_backing_device(&self) -> zbus::Result<zbus::OwnedObjectPath>;

    /// Device property
    #[zbus(property)]
    fn device(&self) -> zbus::Result<Vec<u8>>;

    /// DeviceNumber property
    #[zbus(property)]
    fn device_number(&self) -> zbus::Result<u64>;

    /// Drive property
    #[zbus(property)]
    fn drive(&self) -> zbus::Result<zbus::OwnedObjectPath>;

    /// HintAuto property
    #[zbus(property)]
    fn hint_auto(&self) -> zbus::Result<bool>;

    /// HintIconName property
    #[zbus(property)]
    fn hint_icon_name(&self) -> zbus::Result<String>;

    /// HintIgnore property
    #[zbus(property)]
    fn hint_ignore(&self) -> zbus::Result<bool>;

    /// HintName property
    #[zbus(property)]
    fn hint_name(&self) -> zbus::Result<String>;

    /// HintPartitionable property
    #[zbus(property)]
    fn hint_partitionable(&self) -> zbus::Result<bool>;

    /// HintSymbolicIconName property
    #[zbus(property)]
    fn hint_symbolic_icon_name(&self) -> zbus::Result<String>;

    /// HintSystem property
    #[zbus(property)]
    fn hint_system(&self) -> zbus::Result<bool>;

    /// Id property
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    /// IdLabel property
    #[zbus(property)]
    fn id_label(&self) -> zbus::Result<String>;

    /// IdType property
    #[zbus(property)]
    fn id_type(&self) -> zbus::Result<String>;

    /// IdUUID property
    #[zbus(property, name = "IdUUID")]
    fn id_uuid(&self) -> zbus::Result<String>;

    /// IdUsage property
    #[zbus(property)]
    fn id_usage(&self) -> zbus::Result<String>;

    /// IdVersion property
    #[zbus(property)]
    fn id_version(&self) -> zbus::Result<String>;

    /// MDRaid property
    #[zbus(property, name = "MDRaid")]
    fn mdraid(&self) -> zbus::Result<zbus::OwnedObjectPath>;

    /// MDRaidMember property
    #[zbus(property, name = "MDRaidMember")]
    fn mdraid_member(&self) -> zbus::Result<zbus::OwnedObjectPath>;

    /// PreferredDevice property
    #[zbus(property)]
    fn preferred_device(&self) -> zbus::Result<Vec<u8>>;

    /// ReadOnly property
    #[zbus(property)]
    fn read_only(&self) -> zbus::Result<bool>;

    /// Size property
    #[zbus(property)]
    fn size(&self) -> zbus::Result<u64>;

    /// Symlinks property
    #[zbus(property)]
    fn symlinks(&self) -> zbus::Result<Vec<Vec<u8>>>;

    /// UserspaceMountOptions property
    #[zbus(property)]
    fn userspace_mount_options(&self) -> zbus::Result<Vec<String>>;
}

#[proxy(
    default_service = "org.freedesktop.FileManager1",
    interface = "org.freedesktop.FileManager1",
    assume_defaults = true
)]
pub trait FileManager1 {
    /// ShowFolders method
    fn show_folders(&self, uris: &[&str], startup_id: &str) -> zbus::Result<()>;

    /// ShowItemProperties method
    fn show_item_properties(&self, uris: &[&str], startup_id: &str) -> zbus::Result<()>;

    /// ShowItems method
    fn show_items(&self, uris: &[&str], startup_id: &str) -> zbus::Result<()>;

    /// OpenLocations property
    #[zbus(property)]
    fn open_locations(&self) -> zbus::Result<Vec<String>>;

    /// OpenWindowsWithLocations property
    #[zbus(property)]
    fn open_windows_with_locations(
        &self,
    ) -> zbus::Result<std::collections::HashMap<String, Vec<String>>>;
}
