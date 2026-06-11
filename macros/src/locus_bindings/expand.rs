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
            #variant(#ty),
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
            Msg::#variant(value) => {
                self.#field = value;
                self.changed.mark(Field::#field_variant);
                self.last_error = None;
            }
        }
    });
    let watchers = bindings.iter().map(|binding| {
        let field_name = binding.field.to_string();
        let variant = &binding.variant;
        let source = &binding.source;
        let ty = &binding.ty;
        let input = match &mode {
            ModuleMode::DirectInput => quote! {
                Msg::#variant(value)
            },
            ModuleMode::MappedInput(message) => quote! {
                super::#message(Msg::#variant(value))
            },
        };
        let error_input = match &mode {
            ModuleMode::DirectInput => quote! {
                Msg::WatchFailed {
                    field: #field_name,
                    error: error.to_string(),
                }
            },
            ModuleMode::MappedInput(message) => quote! {
                super::#message(Msg::WatchFailed {
                    field: #field_name,
                    error: error.to_string(),
                })
            },
        };

        quote! {
            {
                let subscription = ::providers::Subscription::new();
                let context = subscription.context();
                subscriptions.push(subscription);
                let update_sender = sender.clone();
                let error_sender = sender.clone();
                ::providers::spawn(async move {
                    let source = ::providers::provider_for::<#ty, _>(#source);
                    let result = ::providers::run_provider(source, context, move |value| {
                        update_sender.input(#input);
                    })
                    .await;

                    if let Err(error) = result {
                        error_sender.input(#error_input);
                    }
                });
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
                WatchFailed {
                    field: &'static str,
                    error: ::std::string::String,
                },
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
                        Msg::WatchFailed { field, error } => {
                            self.last_error = ::std::option::Option::Some(WatchError {
                                field,
                                error,
                            });
                        }
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
    field_idents: &[Ident],
    bindings: &[BindingConfig],
) -> TokenStream {
    let defaults = field_idents.iter().map(|field| {
        quote! {
            #field: ::std::default::Default::default(),
        }
    });
    let message_variants = bindings.iter().map(|binding| {
        let variant = &binding.variant;
        let ty = &binding.ty;
        quote! {
            #variant(#ty),
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
            #module_ident::Msg::#variant(value) => {
                self.#field = value;
                self.changed.mark(#module_ident::Field::#field_variant);
                self.last_error = None;
            }
        }
    });
    let watchers = bindings.iter().map(|binding| {
        let field_name = binding.field.to_string();
        let variant = &binding.variant;
        let source = &binding.source;
        let ty = &binding.ty;

        quote! {
            {
                let subscription = ::providers::Subscription::new();
                let context = subscription.context();
                subscriptions.push(subscription);
                let update_sender = sender.clone();
                let error_sender = sender.clone();
                ::providers::spawn(async move {
                    let source = ::providers::provider_for::<#ty, _>(#source);
                    let result = ::providers::run_provider(source, context, move |value| {
                        update_sender.input(#module_ident::Msg::#variant(value));
                    })
                    .await;

                    if let Err(error) = result {
                        error_sender.input(#module_ident::Msg::WatchFailed {
                            field: #field_name,
                            error: error.to_string(),
                        });
                    }
                });
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
            pub(super) struct Changed {
                mask: ::std::cell::Cell<u128>,
            }

            impl Changed {
                pub(super) fn mark(&self, field: Field) {
                    self.mask.set(self.mask.get() | field.bit());
                }

                pub(super) fn contains(&self, field: Field) -> bool {
                    self.mask.get() & field.bit() != 0
                }

                pub(super) fn clear(&self) {
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
                WatchFailed {
                    field: &'static str,
                    error: ::std::string::String,
                },
            }
        }

        impl ::std::default::Default for #model {
            fn default() -> Self {
                Self {
                    #(#defaults)*
                    last_error: ::std::option::Option::None,
                    changed: #module_ident::Changed::default(),
                    subscriptions: ::providers::SubscriptionGroup::new(),
                }
            }
        }

        impl #model {
            pub fn changed(&self, field: #module_ident::Field) -> bool {
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

            pub fn update(&mut self, msg: #module_ident::Msg) {
                match msg {
                    #(#updates)*
                    #module_ident::Msg::WatchFailed { field, error } => {
                        self.last_error = ::std::option::Option::Some(#module_ident::WatchError {
                            field,
                            error,
                        });
                    }
                }
            }

            pub fn start<Component>(
                sender: ::relm4::ComponentSender<Component>,
            ) -> ::providers::SubscriptionGroup
            where
                Component: ::relm4::Component<Input = #module_ident::Msg> + 'static,
                <Component as ::relm4::Component>::Output: Send,
                <Component as ::relm4::Component>::CommandOutput: Send,
            {
                let mut subscriptions = ::providers::SubscriptionGroup::new();
                #(#watchers)*
                subscriptions
            }
        }
    }
}
