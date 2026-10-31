use quote::{format_ident, quote};
use syn::{Attribute, Data, DeriveInput, Error, Fields, Result};

/// The policy to apply to this field whenever it changes,
/// indicating how the hot-reloading algorithm should handle it.
#[derive(Debug)]
enum HotReloadPolicy {
    /// A change of this property requires the entire bar to be recreated.
    Recreate,
    /// A change of this property can be updated normally.
    Normal(String),
    /// A module list has changed, requiring a full module diff.
    /// If the number of modules has changed, the container is re-rendered.
    Modules(String),
    /// This property is not hot-reloadable and should not be checked.
    Ignore,
}

/// Implementation of `#[derive(HotReload)]`.
/// See [`super::derive_hot_reload`].
pub(crate) fn parse_diff_struct(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let struct_ident = input.ident;
    let diff_struct_ident = format_ident!("{}Diff", struct_ident);

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
    let mut normal_diffs = Vec::new();
    let mut module_diffs = Vec::new();

    let mut normal_diff_fields = Vec::new();
    let mut module_diff_fields = Vec::new();

    let mut normal_diff_empty_checks = Vec::new();
    let mut module_diff_empty_checks = Vec::new();

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

                normal_diff_fields.push(quote! {
                    pub #field_ident: bool,
                });

                normal_diff_empty_checks.push(quote! {
                    self.#field_ident
                });
            }
            HotReloadPolicy::Modules(field) => {
                let field_ident = format_ident!("{field}");

                module_diffs.push(quote! {
                    {
                        let modules_old = self.#field_ident.as_deref().unwrap_or_default();
                        let modules_new = new.#field_ident.as_deref().unwrap_or_default();

                        diff.#field_ident = if modules_old.len() == modules_new.len() {
                            let changed_idx = modules_old
                                .iter()
                                .zip(modules_new)
                                .enumerate()
                                .filter(|(_, (old, new))| old != new)
                                .map(|(i, _)| i)
                                .collect::<Vec<_>>();

                            if changed_idx.is_empty() {
                                crate::config::diff::ModulesDiff::NoChange
                            } else {
                                crate::config::diff::ModulesDiff::Changed(changed_idx)
                            }
                        } else {
                            crate::config::diff::ModulesDiff::Reload
                        };
                    }
                });

                module_diff_fields.push(quote! {
                    pub #field_ident: crate::config::diff::ModulesDiff,
                });

                module_diff_empty_checks.push(quote! {
                    !self.#field_ident.is_empty()
                });
            }
            HotReloadPolicy::Ignore => {}
        }
    }

    let recreate_required = recreate_checks
        .into_iter()
        .reduce(|recreate, check| quote!(#recreate || #check))
        .unwrap_or_else(|| quote!(false));

    let has_changes = normal_diff_empty_checks
        .into_iter()
        .chain(module_diff_empty_checks)
        .reduce(|has_changes, check| quote!(#has_changes || #check))
        .unwrap_or_else(|| quote!(false));

    Ok(quote! {
        impl #struct_ident {
            pub fn hot_reload_diff(&self, new: &Self) ->  crate::config::diff::BarDiffAction {
                let recreate_required = #recreate_required;

                if recreate_required {
                    diff::BarDiffAction::Reload
                } else {
                    let mut diff = crate::config::diff::#diff_struct_ident::default();

                    #(#normal_diffs)*
                    #(#module_diffs)*

                    crate::config::diff::BarDiffAction::Update(diff)
                }
            }
        }

        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct #diff_struct_ident {
            #(#normal_diff_fields)*
            #(#module_diff_fields)*
        }

        impl #diff_struct_ident {
            pub fn is_empty(&self) -> bool {
                !(#has_changes)
            }
        }
    }
    .into())
}

/// Attempts to parse the field attributes to find a `#[hot_reload(...)]` policy,
/// and parse this into the `HotReloadPolicy` enum.
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
