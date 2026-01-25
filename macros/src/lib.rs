use proc_macro::TokenStream;
use syn::{parse_macro_input, Error, DeriveInput};

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