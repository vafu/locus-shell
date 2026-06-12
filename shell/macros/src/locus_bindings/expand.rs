use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Path, Type, Visibility};

use super::config::BindingConfig;

pub(super) enum ModuleMode {
    DirectInput,
    MappedInput(Path),
}

pub(super) fn expand_locus_module(
    visibility: Visibility,
    module_ident: Ident,
    component: Type,
    bindings: Vec<BindingConfig>,
    mode: ModuleMode,
) -> TokenStream {
    let fields = bindings.iter().map(|binding| {
        let field = &binding.field;
        let ty = &binding.ty;
        quote! {
            pub #field: #ty,
        }
    });
    let defaults = bindings.iter().map(|binding| {
        let field = &binding.field;
        quote! {
            #field: ::std::default::Default::default(),
        }
    });
    let message_variants = bindings.iter().map(|binding| {
        let variant = &binding.variant;
        let ty = &binding.ty;
        quote! {
            #variant(::std::result::Result<#ty, ::providers::ProviderError>),
        }
    });
    let field_variants = bindings.iter().map(|binding| {
        let variant = &binding.variant;
        quote! {
            #variant,
        }
    });
    let updates = bindings.iter().map(|binding| {
        let field = &binding.field;
        let variant = &binding.variant;
        let field_variant = &binding.variant;
        quote! {
            Msg::#variant(result) => {
                match result {
                    ::std::result::Result::Ok(value) => {
                        self.#field = value;
                        self.changed.mark(Field::#field_variant);
                        self.last_error = ::std::option::Option::None;
                    }
                    ::std::result::Result::Err(error) => {
                        self.last_error = ::std::option::Option::Some(WatchError {
                            field: stringify!(#field),
                            error: error.to_string(),
                        });
                    }
                }
            }
        }
    });
    let watchers = bindings.iter().map(|binding| {
        let variant = &binding.variant;
        let source = &binding.source;
        let ty = &binding.ty;
        let input = match &mode {
            ModuleMode::DirectInput => quote! {
                Msg::#variant(result)
            },
            ModuleMode::MappedInput(message) => quote! {
                super::#message(Msg::#variant(result))
            },
        };

        quote! {
            {
                let update_sender = sender.clone();
                let source = ::providers::provider_for::<#ty, _>(#source);
                let subscription = ::providers::Subscription::spawn(move |cancellation| async move {
                    ::providers::run_provider(source, cancellation, move |result| {
                        let result = result.map_err(|error| {
                            ::providers::ProviderError::new(error.to_string())
                        });
                        update_sender.input(#input);
                    })
                    .await;
                });
                subscriptions.push(subscription);
            }
        }
    });

    quote! {
        #visibility mod #module_ident {
            #[allow(unused_imports)]
            use super::*;

            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct WatchError {
                pub field: &'static str,
                pub error: ::std::string::String,
            }

            #[derive(Debug)]
            pub struct Model {
                #(#fields)*
                pub last_error: ::std::option::Option<WatchError>,
                changed: Changed,
                subscriptions: ::providers::SubscriptionGroup,
            }

            impl ::std::default::Default for Model {
                fn default() -> Self {
                    Self {
                        #(#defaults)*
                        last_error: ::std::option::Option::None,
                        changed: Changed::default(),
                        subscriptions: ::providers::SubscriptionGroup::new(),
                    }
                }
            }

            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            #[repr(u8)]
            pub enum Field {
                #(#field_variants)*
            }

            #[derive(Debug, Default)]
            struct Changed {
                mask: ::std::cell::Cell<u128>,
            }

            impl Changed {
                fn mark(&self, field: Field) {
                    self.mask.set(self.mask.get() | field.bit());
                }

                fn contains(&self, field: Field) -> bool {
                    self.mask.get() & field.bit() != 0
                }

                fn clear(&self) {
                    self.mask.set(0);
                }
            }

            impl Field {
                const fn bit(self) -> u128 {
                    1 << (self as u8)
                }
            }

            #[derive(Debug)]
            pub enum Msg {
                #(#message_variants)*
            }

            impl Model {
                pub fn changed(&self, field: Field) -> bool {
                    self.changed.contains(field)
                }

                pub fn clear_changed(&self) {
                    self.changed.clear();
                }

                pub fn set_subscriptions(
                    &mut self,
                    subscriptions: ::providers::SubscriptionGroup,
                ) {
                    self.subscriptions = subscriptions;
                }

                pub fn update(&mut self, msg: Msg) {
                    match msg {
                        #(#updates)*
                    }
                }
            }

            pub fn start(
                sender: ::relm4::ComponentSender<super::#component>,
            ) -> ::providers::SubscriptionGroup {
                let mut subscriptions = ::providers::SubscriptionGroup::new();
                #(#watchers)*
                subscriptions
            }
        }
    }
}

pub(super) fn expand_model_impl(
    module_ident: Ident,
    model: &Ident,
    fields: &[(Ident, Type)],
    bindings: &[BindingConfig],
) -> TokenStream {
    let source_local_fields = fields
        .iter()
        .filter(|(field, _ty)| !bindings.iter().any(|binding| binding.field == *field))
        .map(|(field, _ty)| field)
        .collect::<Vec<_>>();
    let context_fields = fields
        .iter()
        .filter(|(field, _ty)| !bindings.iter().any(|binding| binding.field == *field))
        .collect::<Vec<_>>();
    let constructor_args = context_fields.iter().map(|(field, ty)| {
        quote! {
            #field: #ty
        }
    });
    let constructor_values = fields.iter().map(|(field, _ty)| {
        if bindings.iter().any(|binding| binding.field == *field) {
            quote! {
                #field: ::std::default::Default::default(),
            }
        } else {
            quote! {
                #field,
            }
        }
    });
    let default_impl = if context_fields.is_empty() {
        quote! {
            impl ::std::default::Default for #model {
                fn default() -> Self {
                    Self::new()
                }
            }
        }
    } else {
        TokenStream::new()
    };
    let message_variants = bindings.iter().map(|binding| {
        let variant = &binding.variant;
        let ty = &binding.ty;
        quote! {
            #variant(::std::result::Result<#ty, ::providers::ProviderError>),
        }
    });
    let field_variants = bindings.iter().map(|binding| {
        let variant = &binding.variant;
        quote! {
            #variant,
        }
    });
    let updates = bindings.iter().map(|binding| {
        let field = &binding.field;
        let variant = &binding.variant;
        let field_variant = &binding.variant;
        let module_ident = &module_ident;
        quote! {
            #module_ident::Msg::#variant(result) => {
                match result {
                    ::std::result::Result::Ok(value) => {
                        self.#field = value;
                        self.__shell.mark(#module_ident::Field::#field_variant);
                        self.__shell.clear_error();
                    }
                    ::std::result::Result::Err(error) => {
                        self.__shell.set_error(#module_ident::WatchError {
                            field: stringify!(#field),
                            error: error.to_string(),
                        });
                    }
                }
            }
        }
    });
    let watchers = bindings.iter().map(|binding| {
        let variant = &binding.variant;
        let source = &binding.source;
        let ty = &binding.ty;
        let source_locals = source_local_fields.iter().map(|field| {
            quote! {
                let #field = &self.#field;
            }
        });

        quote! {
            {
                let update_sender = sender.clone();
                #(#source_locals)*
                let source = ::providers::provider_for::<#ty, _>(#source);
                let subscription = ::providers::Subscription::spawn(move |cancellation| async move {
                    ::providers::run_provider(source, cancellation, move |result| {
                        let result = result.map_err(|error| {
                            ::providers::ProviderError::new(error.to_string())
                        });
                        let input: <Component as ::relm4::Component>::Input =
                            #module_ident::Msg::#variant(result).into();
                        update_sender.input(input);
                    })
                    .await;
                });
                subscriptions.push(subscription);
            }
        }
    });

    quote! {
        pub mod #module_ident {
            #[allow(unused_imports)]
            use super::*;

            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            #[repr(u8)]
            pub enum Field {
                #(#field_variants)*
            }

            #[derive(Debug, Default)]
            struct Changed {
                mask: ::std::cell::Cell<u128>,
            }

            impl Changed {
                fn mark(&self, field: Field) {
                    self.mask.set(self.mask.get() | field.bit());
                }

                fn contains(&self, field: Field) -> bool {
                    self.mask.get() & field.bit() != 0
                }

                fn clear(&self) {
                    self.mask.set(0);
                }
            }

            impl Field {
                const fn bit(self) -> u128 {
                    1 << (self as u8)
                }
            }

            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct WatchError {
                pub field: &'static str,
                pub error: ::std::string::String,
            }

            #[derive(Debug)]
            pub enum Msg {
                #(#message_variants)*
            }

            #[derive(Debug, Default)]
            pub(super) struct Runtime {
                last_error: ::std::option::Option<WatchError>,
                changed: Changed,
                subscriptions: ::providers::SubscriptionGroup,
            }

            impl Runtime {
                pub(super) fn changed(&self, field: Field) -> bool {
                    self.changed.contains(field)
                }

                pub(super) fn mark(&self, field: Field) {
                    self.changed.mark(field);
                }

                pub(super) fn clear_changed(&self) {
                    self.changed.clear();
                }

                pub(super) fn last_error(&self) -> ::std::option::Option<&WatchError> {
                    self.last_error.as_ref()
                }

                pub(super) fn clear_error(&mut self) {
                    self.last_error = ::std::option::Option::None;
                }

                pub(super) fn set_error(&mut self, error: WatchError) {
                    self.last_error = ::std::option::Option::Some(error);
                }

                pub(super) fn set_subscriptions(
                    &mut self,
                    subscriptions: ::providers::SubscriptionGroup,
                ) {
                    self.subscriptions = subscriptions;
                }
            }
        }

        impl #model {
            pub fn new(#(#constructor_args),*) -> Self {
                Self {
                    #(#constructor_values)*
                    __shell: #module_ident::Runtime::default(),
                }
            }

            pub fn changed(&self, field: #module_ident::Field) -> bool {
                self.__shell.changed(field)
            }

            pub fn clear_changed(&self) {
                self.__shell.clear_changed();
            }

            pub fn last_error(&self) -> ::std::option::Option<&#module_ident::WatchError> {
                self.__shell.last_error()
            }

            pub fn set_subscriptions(
                &mut self,
                subscriptions: ::providers::SubscriptionGroup,
            ) {
                self.__shell.set_subscriptions(subscriptions);
            }

            pub fn update(&mut self, msg: #module_ident::Msg) {
                match msg {
                    #(#updates)*
                }
            }

            pub fn start<Component>(
                &self,
                sender: ::relm4::ComponentSender<Component>,
            ) -> ::providers::SubscriptionGroup
            where
                Component: ::relm4::Component + 'static,
                <Component as ::relm4::Component>::Input:
                    ::std::convert::From<#module_ident::Msg> + Send,
                <Component as ::relm4::Component>::Output: Send,
                <Component as ::relm4::Component>::CommandOutput: Send,
            {
                let mut subscriptions = ::providers::SubscriptionGroup::new();
                #(#watchers)*
                subscriptions
            }
        }

        #default_impl
    }
}
