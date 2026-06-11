use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemMod, parse2};

use super::*;
use crate::locus_bindings::config::ComponentConfig;

#[test]
fn parses_binding_config() {
    let config = parse2::<BindingsConfig>(quote! {
        component = Bar,
        message = BarMsg::Locus,
        selected_window_title: String = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE),
    })
    .unwrap();

    assert_eq!(config.bindings.len(), 1);
    assert_eq!(config.bindings[0].field, "selected_window_title");
    assert_eq!(config.bindings[0].variant, "SelectedWindowTitle");
}

#[test]
fn expands_inline_module() {
    let attr = quote! {
        component = Bar,
        message = BarMsg::Locus,
        selected_window_title: String = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE),
    };
    let item = quote! {
        mod locus {}
    };

    let expanded = expand(attr, item).unwrap();
    let _module: ItemMod = parse2(expanded).unwrap();
}

#[test]
fn expands_component_impl() {
    let attr = quote! {
        selected_window_title: String = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE),
    };
    let item = quote! {
        impl SimpleComponent for Bar {
            type Init = BarInit;
            type Input = locus::Msg;
            type Output = ();

            view! {
                gtk::Window {}
            }

            fn init(
                init: Self::Init,
                root: Self::Root,
                sender: ComponentSender<Self>,
            ) -> ComponentParts<Self> {
                let model = Bar {
                    title: init.title,
                    locus: locus::Model::default(),
                };
                let widgets = view_output!();
                ComponentParts { model, widgets }
            }
        }
    };

    let expanded = expand_component(attr, item).unwrap();
    let source = expanded.to_string();
    assert!(source.contains("mod locus"));
    assert!(source.contains("model . locus . set_subscriptions (locus :: start"));
    assert!(source.contains("fn update"));
    assert!(source.contains("providers :: run_provider"));
    assert!(source.contains("subscriptions : :: providers :: SubscriptionGroup"));
    assert!(source.contains("subscriptions . push (subscription)"));
}

#[test]
fn expands_dbus_property_provider_source() {
    let attr = quote! {
        battery_percent: f64 = BATTERY.bind(Battery::PERCENTAGE),
    };
    let item = component_item();

    let expanded = expand_component(attr, item).unwrap();
    let source = expanded.to_string();

    assert!(source.contains("providers :: run_provider"));
    assert!(source.contains("BATTERY . bind"));
}

#[test]
fn expands_mixed_provider_sources() {
    let attr = quote! {
        selected_window_title: String = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE),
        battery_percent: f64 = BATTERY.bind(Battery::PERCENTAGE),
    };
    let item = component_item();

    let expanded = expand_component(attr, item).unwrap();
    let source = expanded.to_string();

    assert_eq!(source.matches("providers :: run_provider").count(), 2);
}

#[test]
fn expands_locus_view_setters() {
    let attr = quote! {
        selected_window_title: String = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE),
    };
    let item = quote! {
        impl SimpleComponent for Bar {
            type Init = BarInit;
            type Input = locus::Msg;
            type Output = ();

            view! {
                gtk::Window {
                    gtk::Label {
                        #[locus(selected_window_title)]
                        set_label: |title| title.as_str(),

                        #[locus(selected_window_title)]
                        set_css_classes: window_title_classes,
                    }
                }
            }

            fn init(
                init: Self::Init,
                root: Self::Root,
                sender: ComponentSender<Self>,
            ) -> ComponentParts<Self> {
                let model = Bar {
                    title: init.title,
                    locus: locus::Model::default(),
                };
                let widgets = view_output!();
                ComponentParts { model, widgets }
            }
        }
    };

    let expanded = expand_component(attr, item).unwrap();
    let source = expanded.to_string();
    assert!(source.contains("# [track"));
    assert!(source.contains("SelectedWindowTitle"));
    assert!(source.contains("let title = & model . locus . selected_window_title"));
    assert!(source.contains("window_title_classes"));
}

#[test]
fn expands_typed_model() {
    let item = quote! {
        pub struct BarLocus {
            #[locus(
                source = locus_graph::paths::SELECTED_WINDOW
                    .property(locus_graph::model::Window::TITLE)
            )]
            pub selected_window_title: String,
            #[locus(source = DISPLAY_DEVICE.bind(DisplayDevice::PERCENTAGE))]
            pub battery_percent: f64,
        }
    };

    let expanded = expand_model(TokenStream::new(), item).unwrap();
    let source = expanded.to_string();

    assert!(source.contains("pub struct BarLocus"));
    assert!(source.contains("pub mod locus"));
    assert!(source.contains("pub enum Msg"));
    assert!(source.contains("pub enum Field"));
    assert!(source.contains("SelectedWindowTitle"));
    assert!(source.contains("BatteryPercent"));
    assert!(source.contains("changed : locus :: Changed"));
    assert!(source.contains("subscriptions : :: providers :: SubscriptionGroup"));
    assert!(source.contains("subscriptions . push (subscription)"));
    assert!(source.contains("providers :: run_provider"));
}

#[test]
fn expands_model_component_impl() {
    let attr = quote! {
        model = BarLocus
    };
    let item = quote! {
        impl SimpleComponent for Bar {
            type Init = BarInit;
            type Input = locus::Msg;
            type Output = ();

            view! {
                gtk::Window {
                    gtk::Label {
                        #[locus(selected_window_title)]
                        set_label: |title| title.as_str(),
                    }
                }
            }

            fn init(
                init: Self::Init,
                root: Self::Root,
                sender: ComponentSender<Self>,
            ) -> ComponentParts<Self> {
                let model = Bar {
                    title: init.title,
                    locus: BarLocus::default(),
                };
                let widgets = view_output!();
                ComponentParts { model, widgets }
            }
        }
    };

    let expanded = expand_component(attr, item).unwrap();
    let source = expanded.to_string();

    assert!(!source.contains("mod locus"));
    assert!(source.contains("BarLocus :: start"));
    assert!(source.contains("model . locus . set_subscriptions (BarLocus :: start"));
    assert!(source.contains("locus :: Field :: SelectedWindowTitle"));
    assert!(source.contains("fn update"));
}

#[test]
fn rejects_duplicate_binding_fields() {
    let error = component_parse_error(quote! {
        selected_window_title: String = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE),
        selected_window_title: String = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE),
    });

    assert!(error.to_string().contains("duplicate Locus binding field"));
}

#[test]
fn rejects_duplicate_generated_variants() {
    let error = component_parse_error(quote! {
        selected_window_title: String = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE),
        selected__window_title: String = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE),
    });

    assert!(
        error
            .to_string()
            .contains("Locus binding fields must generate unique message variants")
    );
}

#[test]
fn rejects_too_many_bindings_for_dirty_mask() {
    let bindings = (0..129).map(|index| {
        let field = format_ident!("field_{index}");
        quote! {
            #field: String = locus_graph::paths::SELECTED_WINDOW
                .property(locus_graph::model::Window::TITLE),
        }
    });
    let error = component_parse_error(quote! {
        #(#bindings)*
    });

    assert!(
        error
            .to_string()
            .contains("locus components support at most 128 bindings")
    );
}

#[test]
fn accepts_parenthesized_binding_expr() {
    let config = parse2::<BindingsConfig>(quote! {
        component = Bar,
        message = BarMsg::Locus,
        selected_window_title: String = (locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE)),
    })
    .unwrap();
    let expected = quote! {
        locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE)
    };

    let expr = &config.bindings[0].source;
    assert_eq!(quote!(#expr).to_string(), expected.to_string());
}

#[test]
fn treats_sources_as_generic_provider_expressions() {
    let config = parse2::<ComponentConfig>(quote! {
        battery_percent: f64 = BATTERY.bind(Battery::PERCENTAGE),
    })
    .unwrap();

    let source = &config.bindings[0].source;
    assert_eq!(
        quote!(#source).to_string(),
        quote!(BATTERY.bind(Battery::PERCENTAGE)).to_string()
    );
}

fn component_parse_error(tokens: TokenStream) -> syn::Error {
    match parse2::<ComponentConfig>(tokens) {
        Ok(_) => panic!("expected component config parse error"),
        Err(error) => error,
    }
}

fn component_item() -> TokenStream {
    quote! {
        impl SimpleComponent for Bar {
            type Init = BarInit;
            type Input = locus::Msg;
            type Output = ();

            view! {
                gtk::Window {}
            }

            fn init(
                init: Self::Init,
                root: Self::Root,
                sender: ComponentSender<Self>,
            ) -> ComponentParts<Self> {
                let model = Bar {
                    title: init.title,
                    locus: locus::Model::default(),
                };
                let widgets = view_output!();
                ComponentParts { model, widgets }
            }
        }
    }
}
