use proc_macro2::Ident;
use quote::format_ident;
use syn::parse::{Parse, ParseStream};
use syn::{
    Expr, Fields, ItemStruct, Path, Result, Token, Type, parenthesized, punctuated::Punctuated,
};

pub(super) struct BindingsConfig {
    pub(super) component: Path,
    pub(super) message: Path,
    pub(super) bindings: Vec<BindingConfig>,
}

pub(super) struct ComponentConfig {
    pub(super) module: Ident,
    pub(super) model: Option<Type>,
    pub(super) bindings: Vec<BindingConfig>,
}

pub(super) struct ModelConfig {
    pub(super) module: Ident,
}

pub(super) struct BindingConfig {
    pub(super) field: Ident,
    pub(super) variant: Ident,
    pub(super) ty: Type,
    pub(super) source: Expr,
}

impl Parse for BindingsConfig {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut component = None;
        let mut message = None;
        let mut bindings = Vec::new();
        let entries = Punctuated::<ConfigEntry, Token![,]>::parse_terminated(input)?;

        for entry in entries {
            match entry {
                ConfigEntry::Component(path) => component = Some(path),
                ConfigEntry::Message(path) => message = Some(path),
                ConfigEntry::Binding(binding) => bindings.push(binding),
                ConfigEntry::Module(ident) => {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "module is only supported by #[locus_macros::component]",
                    ));
                }
                ConfigEntry::Model(ty) => {
                    return Err(syn::Error::new_spanned(
                        ty,
                        "model is only supported by #[locus_macros::component]",
                    ));
                }
            }
        }

        let component = component.ok_or_else(|| input.error("missing component = Type"))?;
        let message = message.ok_or_else(|| input.error("missing message = Enum::Variant"))?;
        if bindings.is_empty() {
            return Err(input.error("expected at least one Locus binding"));
        }
        validate_bindings(&bindings)?;

        Ok(Self {
            component,
            message,
            bindings,
        })
    }
}

impl Parse for ComponentConfig {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut module = None;
        let mut model = None;
        let mut bindings = Vec::new();
        let entries = Punctuated::<ConfigEntry, Token![,]>::parse_terminated(input)?;

        for entry in entries {
            match entry {
                ConfigEntry::Module(ident) => module = Some(ident),
                ConfigEntry::Binding(binding) => bindings.push(binding),
                ConfigEntry::Model(ty) => model = Some(ty),
                ConfigEntry::Component(path) => {
                    return Err(syn::Error::new_spanned(
                        path,
                        "component is inferred from the annotated impl",
                    ));
                }
                ConfigEntry::Message(path) => {
                    return Err(syn::Error::new_spanned(
                        path,
                        "message is inferred from type Input = locus::Msg",
                    ));
                }
            }
        }

        if model.is_some() && !bindings.is_empty() {
            return Err(input.error(
                "model = Type components read bindings from #[locus_macros::model] fields",
            ));
        }

        if bindings.is_empty() {
            if model.is_none() {
                return Err(input.error("expected model = Type or at least one Locus binding"));
            }
        } else {
            validate_bindings(&bindings)?;
        }

        Ok(Self {
            module: module.unwrap_or_else(|| format_ident!("locus")),
            model,
            bindings,
        })
    }
}

impl Parse for ModelConfig {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.is_empty() {
            return Ok(Self {
                module: format_ident!("locus"),
            });
        }

        let mut module = None;
        let entries = Punctuated::<ConfigEntry, Token![,]>::parse_terminated(input)?;
        for entry in entries {
            match entry {
                ConfigEntry::Module(ident) => module = Some(ident),
                ConfigEntry::Binding(binding) => {
                    return Err(syn::Error::new_spanned(
                        binding.field,
                        "typed model bindings belong on struct fields",
                    ));
                }
                ConfigEntry::Model(ty) => {
                    return Err(syn::Error::new_spanned(
                        ty,
                        "model is inferred from the annotated struct",
                    ));
                }
                ConfigEntry::Component(path) | ConfigEntry::Message(path) => {
                    return Err(syn::Error::new_spanned(
                        path,
                        "only module = ident is supported by #[locus_macros::model]",
                    ));
                }
            }
        }

        Ok(Self {
            module: module.unwrap_or_else(|| format_ident!("locus")),
        })
    }
}

enum ConfigEntry {
    Component(Path),
    Message(Path),
    Module(Ident),
    Model(Type),
    Binding(BindingConfig),
}

impl Parse for ConfigEntry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ident = input.parse::<Ident>()?;
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            if ident == "model" {
                return Ok(Self::Model(input.parse()?));
            }
            let path = input.parse::<Path>()?;
            return match ident.to_string().as_str() {
                "component" => Ok(Self::Component(path)),
                "message" => Ok(Self::Message(path)),
                "module" => {
                    path.get_ident().cloned().map(Self::Module).ok_or_else(|| {
                        syn::Error::new_spanned(path, "module must be an identifier")
                    })
                }
                "model" => unreachable!("model is parsed before path entries"),
                _ => Err(syn::Error::new_spanned(
                    ident,
                    "expected component, message, module, or a typed binding",
                )),
            };
        }

        input.parse::<Token![:]>()?;
        let ty = input.parse::<Type>()?;
        input.parse::<Token![=]>()?;
        let source = parse_binding_expr(input)?;
        let variant = format_ident!("{}", upper_camel(&ident.to_string()));

        Ok(Self::Binding(BindingConfig {
            field: ident,
            variant,
            ty,
            source,
        }))
    }
}

pub(super) fn model_bindings(item: &ItemStruct) -> Result<Vec<BindingConfig>> {
    let Fields::Named(fields) = &item.fields else {
        return Err(syn::Error::new_spanned(
            item,
            "locus models must use named fields",
        ));
    };

    let mut bindings = Vec::new();
    for field in &fields.named {
        let field_ident = field.ident.clone().expect("named field");
        let Some(source) = locus_source(field)? else {
            continue;
        };
        let variant = format_ident!("{}", upper_camel(&field_ident.to_string()));
        bindings.push(BindingConfig {
            field: field_ident,
            variant,
            ty: field.ty.clone(),
            source,
        });
    }

    if bindings.is_empty() {
        return Err(syn::Error::new_spanned(
            item,
            "locus models require at least one #[locus(source = ...)] field",
        ));
    }

    validate_bindings(&bindings)?;
    Ok(bindings)
}

fn locus_source(field: &syn::Field) -> Result<Option<Expr>> {
    for attr in &field.attrs {
        if !attr.path().is_ident("locus") {
            continue;
        }

        let mut source = None;
        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("source") {
                return Err(meta.error("expected source = ..."));
            }
            meta.input.parse::<Token![=]>()?;
            source = Some(parse_binding_expr(meta.input)?);
            Ok(())
        })?;
        return Ok(source);
    }

    Ok(None)
}

fn parse_binding_expr(input: ParseStream<'_>) -> Result<Expr> {
    if input.peek(syn::token::Paren) {
        let content;
        parenthesized!(content in input);
        return content.parse();
    }
    input.parse()
}

fn validate_bindings(bindings: &[BindingConfig]) -> Result<()> {
    if bindings.len() > 128 {
        let field = bindings
            .last()
            .map(|binding| &binding.field)
            .expect("bindings is not empty");
        return Err(syn::Error::new_spanned(
            field,
            "locus components support at most 128 bindings",
        ));
    }

    let mut fields = std::collections::HashSet::new();
    let mut variants = std::collections::HashSet::new();

    for binding in bindings {
        if !fields.insert(binding.field.to_string()) {
            return Err(syn::Error::new_spanned(
                &binding.field,
                "duplicate Locus binding field",
            ));
        }
        if !variants.insert(binding.variant.to_string()) {
            return Err(syn::Error::new_spanned(
                &binding.field,
                "Locus binding fields must generate unique message variants",
            ));
        }
    }

    Ok(())
}

pub(super) fn upper_camel(value: &str) -> String {
    let mut out = String::new();
    for segment in value.split('_').filter(|segment| !segment.is_empty()) {
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.extend(chars);
        }
    }
    out
}
