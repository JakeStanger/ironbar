//! It is necessary to store macros in a separate file due to a compilation error.
//! I believe this stems from the feature flags.
//! Related issue: <https://github.com/rust-lang/rust/issues/81066>

// --- Data Control Device --- \\

#[macro_export]
macro_rules! delegate_data_control_device_manager {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1: smithay_client_toolkit::globals::GlobalData
            ] => $crate::clients::wayland::wlr_data_control::manager::DataControlDeviceManagerState
        );
    };
}

#[macro_export]
macro_rules! delegate_data_control_device {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, udata: [$($udata: ty),*$(,)?]) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::ZwlrDataControlDeviceV1: $udata,
            ] => $crate::clients::wayland::wlr_data_control::manager::DataControlDeviceManagerState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::ZwlrDataControlDeviceV1: $crate::clients::wayland::wlr_data_control::device::DataControlDeviceData
            ] => $crate::clients::wayland::wlr_data_control::manager::DataControlDeviceManagerState
        );
    };
}

#[macro_export]
macro_rules! delegate_data_control_offer {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, udata: [$($udata: ty),*$(,)?]) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::ZwlrDataControlOfferV1: $udata,
            ] => $crate::clients::wayland::wlr_data_control::manager::DataControlDeviceManagerState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::ZwlrDataControlOfferV1: $crate::clients::wayland::wlr_data_control::offer::DataControlOfferData
            ] => $crate::clients::wayland::wlr_data_control::manager::DataControlDeviceManagerState
        );
    };
}

#[macro_export]
macro_rules! delegate_data_control_source {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, udata: [$($udata: ty),*$(,)?]) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::ZwlrDataControlSourceV1: $udata,
            ] => $crate::clients::wayland::wlr_data_control::manager::DataControlDeviceManagerState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::ZwlrDataControlSourceV1: $crate::clients::wayland::wlr_data_control::source::DataControlSourceData
            ] => $crate::clients::wayland::wlr_data_control::manager::DataControlDeviceManagerState
        );
    };
}

// --- Foreign Toplevel --- \\

#[macro_export]
macro_rules! delegate_foreign_toplevel_manager {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1: smithay_client_toolkit::globals::GlobalData
            ] => $crate::clients::wayland::wlr_foreign_toplevel::manager::ToplevelManagerState
        );
    };
}

#[macro_export]
macro_rules! delegate_foreign_toplevel_handle {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, udata: [$($udata: ty),*$(,)?]) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1: $udata,
            ] => $crate::clients::wayland::wlr_foreign_toplevel::manager::ToplevelManagerState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1: $crate::clients::wayland::wlr_foreign_toplevel::handle::ToplevelHandleData
            ] => $crate::clients::wayland::wlr_foreign_toplevel::manager::ToplevelManagerState
        );
    };
}

// --- Workspace --- \\

#[macro_export]
macro_rules! delegate_workspace_manager {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols::ext::workspace::v1::client::ext_workspace_manager_v1::ExtWorkspaceManagerV1: smithay_client_toolkit::globals::GlobalData
            ] => $crate::clients::wayland::ext_workspace::manager::WorkspaceManagerState
        );
    };
}

#[macro_export]
macro_rules! delegate_workspace_group_handle {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, udata: [$($udata: ty),*$(,)?]) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1: $udata,
            ] => $crate::clients::wayland::ext_workspace::manager::WorkspaceManagerState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1: $crate::clients::wayland::ext_workspace::group_handle::WorkspaceGroupHandleData
            ] => $crate::clients::wayland::ext_workspace::manager::WorkspaceManagerState
        );
    };
}

#[macro_export]
macro_rules! delegate_workspace_handle {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, udata: [$($udata: ty),*$(,)?]) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1: $udata,
            ] => $crate::clients::wayland::ext_workspace::manager::WorkspaceManagerState
        );
    };
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty:
            [
                wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1: $crate::clients::wayland::ext_workspace::handle::WorkspaceHandleData
            ] => $crate::clients::wayland::ext_workspace::manager::WorkspaceManagerState
        );
    };
}