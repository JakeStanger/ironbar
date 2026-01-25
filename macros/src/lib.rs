use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Data, DeriveInput, Error, Fields, Result, parse_macro_input};

#[derive(Debug)]
enum HotReloadPolicy {
    Recreate,
    Normal(String),
    Modules(String),
    Ignore,
}

#[proc_macro_derive(HotReload, attributes(hot_reload, hot_reload_diff))]
pub fn derive_hot_reload(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    parse_diff_struct(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn parse_diff_struct(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let struct_ident = input.ident;

    let Data::Struct(data) = input.data else {
        return Err(Error::new_spanned(
            struct_ident,
            "HotReload can only be derived for structs",
        ));
    };

    let Fields::Named(fields) = data.fields else {
        return Err(Error::new_spanned(
            struct_ident,
            "HotReload requires a struct with named fields",
        ));
    };

    if struct_ident != "BarConfig" {
        return Err(Error::new_spanned(
            struct_ident,
            "HotReload can only be derived for BarConfig",
        ));
    }

    let mut recreate_checks = Vec::with_capacity(fields.named.len());
    let mut module_len_checks = Vec::with_capacity(3);
    let mut normal_diffs = Vec::new();
    let mut module_diffs = Vec::new();

    for field in fields.named {
        let Some(field_ident) = field.ident else {
            continue;
        };

        let policy = parse_hot_reload_policy(&field.attrs, &field_ident)?;

        match policy {
            HotReloadPolicy::Recreate => {
                recreate_checks.push(quote! {
                    self.#field_ident != new.#field_ident
                });
            }
            HotReloadPolicy::Normal(field) => {
                let field_ident = format_ident!("{field}");

                normal_diffs.push(quote! {
                    diff.#field_ident = self.#field_ident != new.#field_ident;
                });
            }
            HotReloadPolicy::Modules(field) => {
                let field_ident = format_ident!("{field}");

                module_len_checks.push(quote! {
                    self.#field_ident.as_ref().map(Vec::len).unwrap_or_default()
                        != new.#field_ident.as_ref().map(Vec::len).unwrap_or_default()
                });

                module_diffs.push(quote! {
                    if let (Some(modules_old), Some(modules_new)) =
                        (self.#field_ident.as_ref(), new.#field_ident.as_ref())
                    {
                        diff.#field_ident = modules_old
                            .iter()
                            .zip(modules_new)
                            .enumerate()
                            .filter(|(_, (old, new))| old != new)
                            .map(|(i, _)| i)
                            .collect();
                    }
                });
            }
            HotReloadPolicy::Ignore => {}
        }
    }

    let recreate_required = recreate_checks
        .into_iter()
        .chain(module_len_checks)
        .reduce(|recreate, check| quote!(#recreate || #check))
        .unwrap_or_else(|| quote!(false));

    Ok(quote! {
        impl #struct_ident {
            pub fn hot_reload_diff(&self, new: &Self) ->  crate::config::diff::BarDiff {
                let recreate_required = #recreate_required;

                if recreate_required {
                    diff::BarDiff::Recreate
                } else {
                    let mut diff = crate::config::diff::BarDiffDetails::default();

                    #(#normal_diffs)*
                    #(#module_diffs)*

                     crate::config::diff::BarDiff::Reload(diff)
                }
            }
        }
    }
    .into())
}

fn parse_hot_reload_policy(attrs: &[Attribute], ident: &syn::Ident) -> Result<HotReloadPolicy> {
    let Some(attr) = attrs.iter().find(|attr| attr.path().is_ident("hot_reload")) else {
        return Err(Error::new_spanned(
            ident,
            format!(
                "missing #[hot_reload(...)] policy for field `{ident}`; \
                 use #[hot_reload(recreate)], #[hot_reload(normal)], #[hot_reload(modules)], or #[hot_reload(ignore)]"
            ),
        ));
    };

    let mut policy = None;
    attr.parse_nested_meta(|meta| {
        let path = &meta.path;
        if path.is_ident("recreate") {
            policy = Some(HotReloadPolicy::Recreate);
            Ok(())
        } else if path.is_ident("normal") {
            policy = Some(HotReloadPolicy::Normal(ident.to_string()));
            Ok(())
        } else if path.is_ident("modules") {
            policy = Some(HotReloadPolicy::Modules(ident.to_string()));
            Ok(())
        } else if path.is_ident("ignore") {
            policy = Some(HotReloadPolicy::Ignore);
            Ok(())
        } else {
            Err(meta.error("expected `recreate`, `normal`, `modules`, or `ignore`"))
        }
    })?;

    policy.ok_or_else(|| Error::new_spanned(ident, "unexpected error parsing hot_reload policy"))
}
