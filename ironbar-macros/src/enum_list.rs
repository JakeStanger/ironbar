use quote::quote;
use syn::{Data, DeriveInput, Result};

pub fn parse_enum_list(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let Data::Enum(ref data) = input.data else {
        return Err(syn::Error::new_spanned(input, "Expected an enum"));
    };

    let name = input.ident;

    let names = data
        .variants
        .iter()
        .map(|variant| variant.ident.to_string())
        .map(|name| name.to_lowercase())
        .collect::<Vec<_>>();

    let len = names.len();

    Ok(quote! {
        impl #name {
            pub const fn to_options_array() -> [&'static str; #len] {
                [#(#names),*]
            }
        }
    })
}
