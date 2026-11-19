use zbus::proxy;

#[proxy(
    default_service = "org.freedesktop.UDisks2",
    interface = "org.freedesktop.UDisks2.Manager",
    assume_defaults = true
)]
pub trait Manager {
    /// CanCheck method
    fn can_check(&self, type_: &str) -> zbus::Result<((bool, String),)>;

    /// CanFormat method
    fn can_format(&self, type_: &str) -> zbus::Result<((bool, String),)>;

    /// CanRepair method
    fn can_repair(&self, type_: &str) -> zbus::Result<((bool, String),)>;

    /// CanResize method
    fn can_resize(&self, type_: &str) -> zbus::Result<((bool, u64, String),)>;

    /// EnableModule method
    fn enable_module(&self, name: &str, enable: bool) -> zbus::Result<()>;

    /// EnableModules method
    fn enable_modules(&self, enable: bool) -> zbus::Result<()>;

    /// GetBlockDevices method
    fn get_block_devices(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// GetDrives method
    fn get_drives(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// LoopSetup method
    fn loop_setup(
        &self,
        fd: zbus::zvariant::Fd<'_>,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// MDRaidCreate method
    #[zbus(name = "MDRaidCreate")]
    fn mdraid_create(
        &self,
        blocks: &[&zbus::zvariant::ObjectPath<'_>],
        level: &str,
        name: &str,
        chunk: u64,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// ResolveDevice method
    fn resolve_device(
        &self,
        devspec: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// DefaultEncryptionType property
    #[zbus(property)]
    fn default_encryption_type(&self) -> zbus::Result<String>;

    /// SupportedEncryptionTypes property
    #[zbus(property)]
    fn supported_encryption_types(&self) -> zbus::Result<Vec<String>>;

    /// SupportedFilesystems property
    #[zbus(property)]
    fn supported_filesystems(&self) -> zbus::Result<Vec<String>>;

    /// Version property
    #[zbus(property)]
    fn version(&self) -> zbus::Result<String>;
}

#[proxy(
    default_service = "org.freedesktop.UDisks2",
    interface = "org.freedesktop.UDisks2.Drive",
    assume_defaults = true
)]
pub trait Drive {
    /// Eject method
    fn eject(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// PowerOff method
    fn power_off(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// SetConfiguration method
    fn set_configuration(
        &self,
        value: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// CanPowerOff property
    #[zbus(property)]
    fn can_power_off(&self) -> zbus::Result<bool>;

    /// Configuration property
    #[zbus(property)]
    fn configuration(
        &self,
    ) -> zbus::Result<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>;

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
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<bool>;

    /// Mount method
    fn mount(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<String>;

    /// Repair method
    fn repair(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<bool>;

    /// Resize method
    fn resize(
        &self,
        size: u64,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// SetLabel method
    fn set_label(
        &self,
        label: &str,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// SetUUID method
    #[zbus(name = "SetUUID")]
    fn set_uuid(
        &self,
        uuid: &str,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// TakeOwnership method
    fn take_ownership(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// Unmount method
    fn unmount(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// MountPoints property
    #[zbus(property)]
    fn mount_points(&self) -> zbus::Result<Vec<Vec<u8>>>;

    /// Size property
    #[zbus(property)]
    fn size(&self) -> zbus::Result<u64>;
}

#[proxy(
    interface = "org.freedesktop.UDisks2.Partition",
    default_service = "org.freedesktop.UDisks2"
)]
pub trait Partition {
    /// Delete method
    fn delete(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// Resize method
    fn resize(
        &self,
        size: u64,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// SetFlags method
    fn set_flags(
        &self,
        flags: u64,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// SetName method
    fn set_name(
        &self,
        name: &str,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// SetType method
    fn set_type(
        &self,
        type_: &str,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// SetUUID method
    #[zbus(name = "SetUUID")]
    fn set_uuid(
        &self,
        uuid: &str,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// Flags property
    #[zbus(property)]
    fn flags(&self) -> zbus::Result<u64>;

    /// IsContained property
    #[zbus(property)]
    fn is_contained(&self) -> zbus::Result<bool>;

    /// IsContainer property
    #[zbus(property)]
    fn is_container(&self) -> zbus::Result<bool>;

    /// Name property
    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;

    /// Number property
    #[zbus(property)]
    fn number(&self) -> zbus::Result<u32>;

    /// Offset property
    #[zbus(property)]
    fn offset(&self) -> zbus::Result<u64>;

    /// Size property
    #[zbus(property)]
    fn size(&self) -> zbus::Result<u64>;

    /// Table property
    #[zbus(property)]
    fn table(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// Type property
    #[zbus(property)]
    fn type_(&self) -> zbus::Result<String>;

    /// UUID property
    #[zbus(property, name = "UUID")]
    fn uuid(&self) -> zbus::Result<String>;
}

#[proxy(
    interface = "org.freedesktop.UDisks2.Block",
    default_service = "org.freedesktop.UDisks2"
)]
pub trait Block {
    /// AddConfigurationItem method
    fn add_configuration_item(
        &self,
        item: &(
            &str,
            std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
        ),
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// Format method
    fn format(
        &self,
        type_: &str,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// GetSecretConfiguration method
    fn get_secret_configuration(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<
        Vec<(
            String,
            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        )>,
    >;

    /// OpenDevice method
    fn open_device(
        &self,
        mode: &str,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<zbus::zvariant::OwnedFd>;

    /// OpenForBackup method
    fn open_for_backup(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<zbus::zvariant::OwnedFd>;

    /// OpenForBenchmark method
    fn open_for_benchmark(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<zbus::zvariant::OwnedFd>;

    /// OpenForRestore method
    fn open_for_restore(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<zbus::zvariant::OwnedFd>;

    /// RemoveConfigurationItem method
    fn remove_configuration_item(
        &self,
        item: &(
            &str,
            std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
        ),
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// Rescan method
    fn rescan(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// RestoreEncryptedHeader method
    fn restore_encrypted_header(
        &self,
        backup_file: &str,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// UpdateConfigurationItem method
    fn update_configuration_item(
        &self,
        old_item: &(
            &str,
            std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
        ),
        new_item: &(
            &str,
            std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
        ),
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// Configuration property
    #[zbus(property)]
    fn configuration(
        &self,
    ) -> zbus::Result<
        Vec<(
            String,
            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        )>,
    >;

    /// CryptoBackingDevice property
    #[zbus(property)]
    fn crypto_backing_device(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// Device property
    #[zbus(property)]
    fn device(&self) -> zbus::Result<Vec<u8>>;

    /// DeviceNumber property
    #[zbus(property)]
    fn device_number(&self) -> zbus::Result<u64>;

    /// Drive property
    #[zbus(property)]
    fn drive(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

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
    fn mdraid(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// MDRaidMember property
    #[zbus(property, name = "MDRaidMember")]
    fn mdraid_member(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

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
    default_service = "org.freedesktop.UDisks2",
    interface = "org.freedesktop.UDisks2.NVMe.Controller",
    assume_defaults = true
)]
pub trait Controller {
    /// SanitizeStart method
    fn sanitize_start(
        &self,
        action: &str,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// SmartGetAttributes method
    fn smart_get_attributes(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>;

    /// SmartSelftestAbort method
    fn smart_selftest_abort(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// SmartSelftestStart method
    fn smart_selftest_start(
        &self,
        type_: &str,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// SmartUpdate method
    fn smart_update(
        &self,
        options: std::collections::HashMap<&str, &zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// ControllerID property
    #[zbus(property, name = "ControllerID")]
    fn controller_id(&self) -> zbus::Result<u16>;

    /// FGUID property
    #[zbus(property, name = "FGUID")]
    fn fguid(&self) -> zbus::Result<String>;

    /// NVMeRevision property
    #[zbus(property, name = "NVMeRevision")]
    fn nvme_revision(&self) -> zbus::Result<String>;

    /// SanitizePercentRemaining property
    #[zbus(property)]
    fn sanitize_percent_remaining(&self) -> zbus::Result<i32>;

    /// SanitizeStatus property
    #[zbus(property)]
    fn sanitize_status(&self) -> zbus::Result<String>;

    /// SmartCriticalWarning property
    #[zbus(property)]
    fn smart_critical_warning(&self) -> zbus::Result<Vec<String>>;

    /// SmartPowerOnHours property
    #[zbus(property)]
    fn smart_power_on_hours(&self) -> zbus::Result<u64>;

    /// SmartSelftestPercentRemaining property
    #[zbus(property)]
    fn smart_selftest_percent_remaining(&self) -> zbus::Result<i32>;

    /// SmartSelftestStatus property
    #[zbus(property)]
    fn smart_selftest_status(&self) -> zbus::Result<String>;

    /// SmartTemperature property
    #[zbus(property)]
    fn smart_temperature(&self) -> zbus::Result<u16>;

    /// SmartUpdated property
    #[zbus(property)]
    fn smart_updated(&self) -> zbus::Result<u64>;

    /// State property
    #[zbus(property)]
    fn state(&self) -> zbus::Result<String>;

    /// SubsystemNQN property
    #[zbus(property, name = "SubsystemNQN")]
    fn subsystem_nqn(&self) -> zbus::Result<Vec<u8>>;

    /// UnallocatedCapacity property
    #[zbus(property)]
    fn unallocated_capacity(&self) -> zbus::Result<u64>;
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
