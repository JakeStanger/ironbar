use proc_macro::TokenStream;
use syn::{DeriveInput, Error, parse_macro_input};

mod enum_list;
mod hot_reload;

/// Derive macro specifically for the `BarConfig` struct.
///
/// Automatically enforces and generates the bar-level diffing check,
/// ensuring all fields are covered.
/// Also generates the diff struct.
#[proc_macro_derive(HotReload, attributes(hot_reload, hot_reload_diff))]
pub fn derive_hot_reload(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    hot_reload::parse_diff_struct(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(EnumList)]
pub fn derive_enum_list(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    enum_list::parse_enum_list(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
