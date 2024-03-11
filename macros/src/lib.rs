extern crate proc_macro;

use global_counter::primitive::exact::CounterU64;
use proc_macro::{Group, TokenStream, TokenTree};
use quote::quote;
use std::iter::FromIterator;
use syn::{self, parse_macro_input, Lit, Meta, MetaNameValue, NestedMeta};

static ID_COUNTER: CounterU64 = CounterU64::new(0);

/// TODO
///
/// Assumes the `style` module is in scope, when using the `Styled` attribute.
///
/// e.g. `#[component(State = "ButtonState", Styled)]`
/// e.g. `#[component(State = "StateType", Styled = "ComponentNameOverride")]`
#[proc_macro_attribute]
pub fn component(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as syn::AttributeArgs);

    let is_styled = attr.iter().any(|v| {
        if let NestedMeta::Meta(m) = v {
            m.path().segments.last().unwrap().ident == "Styled"
        } else {
            false
        }
    });
    let is_internal = attr.iter().any(|v| {
        if let NestedMeta::Meta(m) = v {
            m.path().segments.last().unwrap().ident == "Internal"
        } else {
            false
        }
    });
    let state_type = attr
        .iter()
        .find_map(|v| {
            if let NestedMeta::Meta(m) = v {
                match m {
                    Meta::NameValue(MetaNameValue {
                        path,
                        lit: Lit::Str(s),
                        ..
                    }) if path.segments.last().unwrap().ident == "State" => Some(s.value()),
                    _ => None,
                }
            } else {
                None
            }
        })
        .as_ref()
        .map(|t| proc_macro2::Ident::new(t, proc_macro2::Span::call_site()));

    let component_name_override = attr.iter().find_map(|v| {
        if let NestedMeta::Meta(m) = v {
            match m {
                Meta::NameValue(MetaNameValue {
                    path,
                    lit: Lit::Str(s),
                    ..
                }) if path.segments.last().unwrap().ident == "Styled" => Some(s.value()),
                _ => None,
            }
        } else {
            None
        }
    });

    let style_override_ref = if is_internal {
        quote! { crate::style::StyleOverride }
    } else {
        quote! { style::StyleOverride }
    };

    let styled_ref = if is_internal {
        quote! { crate::style::Styled }
    } else {
        quote! { style::Styled }
    };

    // Add in fields
    let mut i: Vec<_> = input.clone().into_iter().collect();
    if let Some(TokenTree::Group(g)) = i.last() {
        let mut s = g.stream();
        let len = i.len();
        if let Some(state) = &state_type {
            let state_field = quote! {
                state: Option<#state>,
                dirty: bool,
            };

            s.extend(TokenStream::from(state_field));
        }
        if is_styled {
            let styled_fields = quote! {
                class: Option<&'static str>,
                style_overrides: #style_override_ref
            };

            s.extend(TokenStream::from(styled_fields));
        }
        i[len - 1] = TokenTree::Group(Group::new(g.delimiter(), s));
    }
    let mut struct_def = TokenStream::from_iter(i);

    // State impl
    let input = parse_macro_input!(input as syn::ItemStruct);
    let struct_name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    if let Some(state) = &state_type {
        let expanded = quote!(
            impl #impl_generics #struct_name #ty_generics #where_clause {
                #[allow(dead_code)]
                fn state_mut(&mut self) -> &mut #state {
                    self.dirty = true;
                    self.state.as_mut().expect(&format!("Expected state to exist"))
                }

                #[allow(dead_code)]
                fn state_ref(&self) -> & #state {
                    self.state.as_ref().expect(&format!("Expected state to exist"))
                }
            }
        );
        struct_def.extend(TokenStream::from(expanded));
    }

    // Styled impl
    if is_styled {
        let component_name = if let Some(n) = component_name_override {
            n
        } else {
            struct_name.to_string()
        };
        let component_name = component_name.as_str();
        let expanded = quote!(
            impl #impl_generics #styled_ref for #struct_name #ty_generics #where_clause {
                fn name() -> &'static str {
                    #component_name
                }
                fn class(&self) -> Option<&'static str> {
                    self.class
                }
                fn class_mut(&mut self) -> &mut Option<&'static str> {
                    &mut self.class
                }
                fn style_overrides(&self) -> & #style_override_ref {
                    &self.style_overrides
                }
                fn style_overrides_mut(&mut self) -> &mut #style_override_ref {
                    &mut self.style_overrides
                }
            }
        );
        struct_def.extend(TokenStream::from(expanded));
    }

    struct_def
}

/// TODO
#[proc_macro_attribute]
pub fn state_component_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as syn::AttributeArgs);
    let state_type = attr.first().unwrap();

    let expanded = quote! {
        fn replace_state(&mut self, other_state: Box<dyn std::any::Any>) {
            if let Ok(s) = other_state.downcast::<#state_type>() {
                self.state = Some(*s);
            }
        }

        fn take_state(&mut self) -> Option<Box<dyn std::any::Any>> {
            if let Some(s) = self.state.take() {
                Some(Box::new(s))
            } else {
                None
            }
        }

        fn is_dirty(&mut self) -> bool {
            let d = self.dirty;
            self.dirty = false;
            d
        }
    };

    let mut i: Vec<_> = input.into_iter().collect();
    if let Some(TokenTree::Group(g)) = i.last() {
        let mut s = g.stream();
        let len = i.len();
        s.extend(TokenStream::from(expanded));
        i[len - 1] = TokenTree::Group(Group::new(g.delimiter(), s));
    }

    TokenStream::from_iter(i)
}

/// Used by the `node` macro, to generate node keys.
#[proc_macro]
pub fn static_id(_item: TokenStream) -> TokenStream {
    let id = ID_COUNTER.inc();
    quote! { #id }.into()
}
