mod component;
mod config;
mod expand;
mod view;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident, ItemMod, ItemStruct, Result, Type, parse_quote, parse2};

use config::{BindingsConfig, ModelConfig};
use expand::{ModuleMode, expand_locus_module, expand_model_impl};

pub fn expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let config = parse2::<BindingsConfig>(attr)?;
    let module = parse2::<ItemMod>(item)?;
    let visibility = module.vis;
    let module_ident = module.ident;

    if module.content.is_none() {
        return Err(syn::Error::new_spanned(
            module_ident,
            "locus binding modules must use inline module syntax: mod locus {}",
        ));
    }

    Ok(expand_locus_module(
        visibility,
        module_ident,
        Type::Path(syn::TypePath {
            qself: None,
            path: config.component,
        }),
        config.bindings,
        ModuleMode::MappedInput(config.message),
    ))
}

pub fn expand_component(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    component::expand(attr, item)
}

pub fn expand_model(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let config = parse2::<ModelConfig>(attr)?;
    let mut item = parse2::<ItemStruct>(item)?;
    let bindings = config::model_bindings(&item)?;
    let model = item.ident.clone();
    let field_idents = model_field_idents(&item)?;
    let module = config.module;

    strip_locus_field_attrs(&mut item);
    push_generated_model_fields(&mut item, &module)?;

    let generated = expand_model_impl(module, &model, &field_idents, &bindings);

    Ok(quote! {
        #item
        #generated
    })
}

fn model_field_idents(item: &ItemStruct) -> Result<Vec<Ident>> {
    let Fields::Named(fields) = &item.fields else {
        return Err(syn::Error::new_spanned(
            item,
            "locus models must use named fields",
        ));
    };

    fields
        .named
        .iter()
        .map(|field| {
            field
                .ident
                .clone()
                .ok_or_else(|| syn::Error::new_spanned(field, "locus models must use named fields"))
        })
        .collect()
}

fn strip_locus_field_attrs(item: &mut ItemStruct) {
    let Fields::Named(fields) = &mut item.fields else {
        return;
    };

    for field in &mut fields.named {
        field.attrs.retain(|attr| !attr.path().is_ident("locus"));
    }
}

fn push_generated_model_fields(item: &mut ItemStruct, module: &Ident) -> Result<()> {
    let Fields::Named(fields) = &mut item.fields else {
        return Err(syn::Error::new_spanned(
            item,
            "locus models must use named fields",
        ));
    };

    for field in &fields.named {
        let Some(ident) = &field.ident else {
            continue;
        };
        if ident == "last_error" || ident == "changed" || ident == "subscriptions" {
            return Err(syn::Error::new_spanned(
                ident,
                "locus models reserve last_error, changed, and subscriptions",
            ));
        }
    }

    fields.named.push(parse_quote! {
        pub last_error: ::std::option::Option<#module::WatchError>
    });
    fields.named.push(parse_quote! {
        changed: #module::Changed
    });
    fields.named.push(parse_quote! {
        subscriptions: ::providers::SubscriptionGroup
    });

    Ok(())
}

#[cfg(test)]
#[path = "test.rs"]
mod tests;
