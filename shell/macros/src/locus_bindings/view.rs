use proc_macro2::{Delimiter, Group, Ident, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{Expr, ImplItem, ItemImpl, Pat, Result, parse2};

use super::config::BindingConfig;

pub(super) enum ViewBindings<'a> {
    Known(&'a [BindingConfig]),
    Model,
}

enum ViewBinding {
    Known { field: Ident, variant: Ident },
    Model { field: Ident, variant: Ident },
}

impl ViewBinding {
    const fn field(&self) -> &Ident {
        match self {
            Self::Known { field, .. } | Self::Model { field, .. } => field,
        }
    }
}

pub(super) fn transform_locus_view_attributes(
    item_impl: &mut ItemImpl,
    module_ident: &Ident,
    bindings: ViewBindings<'_>,
) -> Result<()> {
    for item in &mut item_impl.items {
        let ImplItem::Macro(item_macro) = item else {
            continue;
        };
        if item_macro.mac.path.is_ident("view") {
            item_macro.mac.tokens =
                transform_tokens(item_macro.mac.tokens.clone(), module_ident, &bindings)?;
        }
    }
    Ok(())
}

fn transform_tokens(
    tokens: TokenStream,
    module_ident: &Ident,
    bindings: &ViewBindings<'_>,
) -> Result<TokenStream> {
    let mut output = Vec::new();
    let mut iter = tokens.into_iter().peekable();

    while let Some(token) = iter.next() {
        if let Some(field) = binding_attr_field(&token, iter.peek())? {
            iter.next();
            let binding = view_binding(field, bindings)?;
            append_locus_tracked_setter(&mut output, &mut iter, module_ident, binding)?;
            continue;
        }

        output.push(transform_token(token, module_ident, bindings)?);
    }

    Ok(output.into_iter().collect())
}

fn transform_token(
    token: TokenTree,
    module_ident: &Ident,
    bindings: &ViewBindings<'_>,
) -> Result<TokenTree> {
    let TokenTree::Group(group) = token else {
        return Ok(token);
    };
    let mut transformed = Group::new(
        group.delimiter(),
        transform_tokens(group.stream(), module_ident, bindings)?,
    );
    transformed.set_span(group.span());
    Ok(TokenTree::Group(transformed))
}

fn view_binding(field: Ident, bindings: &ViewBindings<'_>) -> Result<ViewBinding> {
    match bindings {
        ViewBindings::Known(bindings) => {
            let binding = bindings
                .iter()
                .find(|binding| binding.field == field)
                .ok_or_else(|| {
                    syn::Error::new_spanned(field, "unknown provider field in view attribute")
                })?;
            Ok(ViewBinding::Known {
                field: binding.field.clone(),
                variant: binding.variant.clone(),
            })
        }
        ViewBindings::Model => {
            let variant = format_ident!("{}", super::config::upper_camel(&field.to_string()));
            Ok(ViewBinding::Model { field, variant })
        }
    }
}

fn binding_attr_field(current: &TokenTree, next: Option<&TokenTree>) -> Result<Option<Ident>> {
    let TokenTree::Punct(punct) = current else {
        return Ok(None);
    };
    if punct.as_char() != '#' {
        return Ok(None);
    }

    let Some(TokenTree::Group(group)) = next else {
        return Ok(None);
    };
    if group.delimiter() != Delimiter::Bracket {
        return Ok(None);
    }

    let mut attr_tokens = group.stream().into_iter();
    let Some(TokenTree::Ident(attr_name)) = attr_tokens.next() else {
        return Ok(None);
    };
    let attr = attr_name.to_string();
    if attr != "locus" && attr != "bind" {
        return Ok(None);
    }
    let expected = format!("#[{}(field)]", attr);
    let Some(TokenTree::Group(args)) = attr_tokens.next() else {
        return Err(syn::Error::new_spanned(attr_name, expected));
    };
    if args.delimiter() != Delimiter::Parenthesis {
        return Err(syn::Error::new_spanned(attr_name, expected));
    }
    let mut args = args.stream().into_iter();
    let Some(TokenTree::Ident(field)) = args.next() else {
        return Err(syn::Error::new_spanned(attr_name, expected));
    };
    if args.next().is_some() || attr_tokens.next().is_some() {
        return Err(syn::Error::new_spanned(
            field,
            "expected exactly one provider field",
        ));
    }
    Ok(Some(field))
}

fn append_locus_tracked_setter(
    output: &mut Vec<TokenTree>,
    iter: &mut std::iter::Peekable<impl Iterator<Item = TokenTree>>,
    module_ident: &Ident,
    binding: ViewBinding,
) -> Result<()> {
    let mut setter_tokens = Vec::new();

    loop {
        let Some(token) = iter.next() else {
            return Err(syn::Error::new_spanned(
                binding.field(),
                "expected setter after provider binding attribute",
            ));
        };
        let is_colon = matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ':');
        setter_tokens.push(transform_token(
            token,
            module_ident,
            &ViewBindings::Known(&[]),
        )?);
        if is_colon {
            break;
        }
    }

    let mut adapter_tokens = Vec::new();
    let mut depth = 0usize;
    for token in iter.by_ref() {
        let is_top_level_comma =
            depth == 0 && matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',');
        if is_top_level_comma {
            break;
        }
        match &token {
            TokenTree::Group(_) => adapter_tokens.push(transform_token(
                token,
                module_ident,
                &ViewBindings::Known(&[]),
            )?),
            TokenTree::Punct(punct) if matches!(punct.as_char(), '(' | '[' | '{') => {
                depth += 1;
                adapter_tokens.push(token);
            }
            TokenTree::Punct(punct) if matches!(punct.as_char(), ')' | ']' | '}') && depth > 0 => {
                depth -= 1;
                adapter_tokens.push(token);
            }
            _ => adapter_tokens.push(token),
        }
    }

    let adapter: Expr = parse2(adapter_tokens.into_iter().collect())?;
    let field = binding.field();
    let value_expr = locus_setter_value_expr(adapter, module_ident, field)?;
    output.extend(track_attribute(&binding, module_ident));
    output.extend(setter_tokens);
    output.extend(quote! { #value_expr, });
    Ok(())
}

fn track_attribute(binding: &ViewBinding, module_ident: &Ident) -> TokenStream {
    match binding {
        ViewBinding::Known { variant, .. } => {
            quote! {
                #[track(model.#module_ident.changed(#module_ident::Field::#variant))]
            }
        }
        ViewBinding::Model { variant, .. } => {
            quote! {
                #[track(model.#module_ident.changed(#module_ident::Field::#variant))]
            }
        }
    }
}

fn locus_setter_value_expr(
    adapter: Expr,
    module_ident: &Ident,
    field: &Ident,
) -> Result<TokenStream> {
    let Expr::Closure(closure) = adapter else {
        return Ok(quote! {
            (#adapter)(&model.#module_ident.#field)
        });
    };

    if closure.inputs.len() != 1 {
        return Err(syn::Error::new_spanned(
            closure.or1_token,
            "provider setter closures must accept exactly one field value",
        ));
    }

    let input = closure
        .inputs
        .first()
        .expect("closure input exists")
        .clone();
    validate_locus_value_pat(&input)?;
    let body = closure.body;

    Ok(quote! {
        {
            let #input = &model.#module_ident.#field;
            #body
        }
    })
}

fn validate_locus_value_pat(input: &Pat) -> Result<()> {
    if matches!(input, Pat::Type(_)) {
        return Ok(());
    }

    let Pat::Ident(_) = input else {
        return Err(syn::Error::new_spanned(
            input,
            "provider setter closure parameters must be identifiers or typed patterns",
        ));
    };

    Ok(())
}
