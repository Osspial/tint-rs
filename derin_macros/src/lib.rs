// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![feature(nll)]

// Quote recurses a lot.
#![recursion_limit="256"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use self::proc_macro::TokenStream;

use syn::*;
use quote::{Tokens, ToTokens};

#[proc_macro_derive(WidgetContainer, attributes(derin))]
pub fn derive_widget_container(input_tokens: TokenStream) -> TokenStream {
    let input = input_tokens.to_string();
    let item = syn::parse_derive_input(&input).expect("Attempted derive on non-item");

    let output = impl_widget_container(&item).parse().unwrap();
    output
}

fn impl_widget_container(derive_input: &DeriveInput) -> Tokens {
    let DeriveInput{
        ref ident,
        ref body,
        ref generics,
        ..
    } = *derive_input;

    // Process attributes on the fields in the item being derived
    let mut widget_fields = Vec::new();
    match *body {
        Body::Struct(ref variant_data) =>
            for field in variant_data.fields().iter() {
                let mut widget_field = WidgetField::Widget(field);
                derin_attribute_iter(&field.attrs, |attr| {
                    match *attr {
                        MetaItem::NameValue(ref attr_name, Lit::Str(ref collection_inner, _))
                            if attr_name == "collection" =>
                                match widget_field {
                                    WidgetField::Widget(_) => widget_field = WidgetField::Collection(field, syn::parse_type(collection_inner).expect("Malformed collection type")),
                                    WidgetField::Collection(_, _) => panic!("Repeated #[derin(collection)] attribute")
                                },
                        _ => panic!("Bad Derin attribute: {}", quote!(#attr).to_string())
                    }
                });

                widget_fields.push(widget_field);
            },
        _ => unimplemented!()
    }

    // let parent_mut = parent_mut(derive_input, &action_ty, &widget_fields, &layout_ident);
    // let parent = parent(derive_input, &widget_fields, &layout_ident);

    let dummy_const = Ident::new(format!("_IMPL_PARENT_FOR_{}", ident));

    let generics_expanded = expand_generics(generics, &widget_fields);
    let (impl_generics, _, where_clause) = generics_expanded.split_for_impl();
    let (_, ty_generics, _) = generics.split_for_impl();

    let widget_trait_ty = quote!(Widget);
    let mut widget_ty = None;
    for ty in field_types(widget_fields.iter()) {
        let mut ty_tokens = Tokens::new();
        ty.to_tokens(&mut ty_tokens);

        match widget_ty {
            None => widget_ty = Some(ty_tokens),
            Some(ref t) if *t != ty_tokens => {
                widget_ty = Some(widget_trait_ty.clone());
                break;
            },
            _ => ()
        }
    }

    let call_child_iter = CallChildIter {
        fields: widget_fields.iter().cloned(),
        field_num: 0,
        is_mut: false
    };

    let call_child_mut_iter = CallChildIter {
        fields: widget_fields.iter().cloned(),
        field_num: 0,
        is_mut: true
    };

    let num_children_iter = widget_fields.iter().cloned().enumerate().map(|(field_num, widget_field)| {
        let widget_ident = widget_field.ident().clone().unwrap_or(Ident::new(field_num));
        match widget_field {
            WidgetField::Widget(_) => quote!(+ 1),
            WidgetField::Collection(_, _) => quote!(+ (&self.#widget_ident).into_iter().count())
        }
    });

    let ident_arc_iter = widget_fields.iter().cloned().filter_map(|widget_field| {
        match widget_field.ident().clone() {
            Some(ident) => {
                let tl_ident = thread_local_ident(ident.clone());
                Some(quote!(static #tl_ident: Arc<str> = Arc::from(stringify!(#ident));))
            }
            None => None
        }
    });

    quote!{
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications, unused_imports)]
        const #dummy_const: () = {mod import {
            extern crate derin as _derive_derin;
            use self::_derive_derin::LoopFlow;
            use self::_derive_derin::container::WidgetContainer;
            use self::_derive_derin::widgets::custom::{Widget, WidgetInfo, WidgetInfoMut};
            use self::_derive_derin::gl_render::RenderFrame;
            use std::sync::Arc;
            use super::*;

            // Ideally we'd be using lazy_static, but macro re-exporting doesn't work. Maybe we'll
            // do this when declarative macros 2.0 gets stable.
            thread_local!{
                #(#ident_arc_iter)*
            }

            #[automatically_derived]
            impl #impl_generics WidgetContainer<__S> for #ident #ty_generics #where_clause {
                #[inline]
                fn num_children(&self) -> usize {
                    0 #(#num_children_iter)*
                }

                #[allow(unused_assignments, unused_variables, unused_mut)]
                fn framed_children<'a, __F, __G>(&'a self, mut for_each_child: __G)
                    where __G: FnMut(WidgetInfo<'a, __F, __S>) -> LoopFlow,
                          __F: 'a + RenderFrame
                {
                    let mut index = 0;
                    #(#call_child_iter)*
                }

                #[allow(unused_assignments, unused_variables, unused_mut)]
                fn framed_children_mut<'a, __F, __G>(&'a mut self, mut for_each_child: __G)
                    where __G: FnMut(WidgetInfoMut<'a, __F, __S>) -> LoopFlow,
                          __F: 'a + RenderFrame
                {
                    let mut index = 0;
                    #(#call_child_mut_iter)*
                }
            }
        }};
    }
}

fn thread_local_ident(ident: Ident) -> Ident {
    let mut tl_ident_str = "TL_IDENT_ARC_".to_string();
    tl_ident_str.push_str(ident.as_ref());
    Ident::from(tl_ident_str)
}

struct CallChildIter<'a, W>
        where W: Iterator<Item = WidgetField<'a>>
{
    fields: W,
    field_num: u32,
    is_mut: bool
}

impl<'a, W> Iterator for CallChildIter<'a, W>
        where W: Iterator<Item = WidgetField<'a>>
{
    type Item = Tokens;

    fn next(&mut self) -> Option<Tokens> {
        if let Some(widget_field) = self.fields.next() {
            let widget_ident = widget_field.ident().clone().unwrap_or(Ident::new(self.field_num as usize));
            let tl_ident = thread_local_ident(widget_ident.clone());
            let widget_expr = match self.is_mut {
                true => quote!(&mut self.#widget_ident),
                false => quote!(&self.#widget_ident)
            };
            let new_summary = match self.is_mut {
                true => quote!(_derive_derin::widgets::custom::WidgetInfoMut::new),
                false => quote!(_derive_derin::widgets::custom::WidgetInfo::new),
            };

            let output: Tokens;

            match widget_field {
                WidgetField::Widget(field) => {
                    let child_id = match field.ident {
                        Some(_) => quote!(_derive_derin::widgets::custom::WidgetIdent::Str(#tl_ident.with(|i| i.clone()))),
                        None => quote!(_derive_derin::widgets::custom::WidgetIdent::Num(#widget_ident))
                    };

                    output = quote!{{
                        let flow = for_each_child(#new_summary (#child_id, index, #widget_expr));
                        if let LoopFlow::Break = flow {
                            return;
                        }
                        index += 1;
                    }};
                },
                WidgetField::Collection(field, _) => {
                    let child_id = match field.ident {
                        Some(_) => quote!(_derive_derin::widgets::custom::WidgetIdent::StrCollection(#tl_ident.with(|i| i.clone()), child_index as u32)),
                        None => quote!(_derive_derin::widgets::custom::WidgetIdent::NumCollection(#widget_ident, child_index as u32))
                    };

                    output = quote!{{
                        for (child_index, child) in (#widget_expr).into_iter().enumerate() {
                            let flow = for_each_child(#new_summary (#child_id, index, child));

                            if let LoopFlow::Break = flow {
                                return;
                            }
                            index += 1;
                        }
                    }}
                }
            }

            self.field_num += 1;
            Some(output)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
enum WidgetField<'a> {
    Widget(&'a Field),
    Collection(&'a Field, Ty)
}

impl<'a> WidgetField<'a> {
    fn ident(&self) -> &'a Option<Ident> {
        match *self {
            WidgetField::Widget(field) |
            WidgetField::Collection(field, _) => &field.ident
        }
    }
}

fn derin_attribute_iter<F>(attrs: &[Attribute], mut for_each: F)
        where F: FnMut(&MetaItem)
{
    for attr in attrs.iter().filter(|attr| attr.name() == "derin") {
        if let MetaItem::List(_, ref meta_list) = attr.value {
            for inner_attr in meta_list.iter() {
                if let NestedMetaItem::MetaItem(ref inner_meta) = *inner_attr {
                    for_each(inner_meta)
                } else {
                    panic!("Invalid derin attribute: {}", quote!(#attr).to_string())
                }
            }
        } else {
            panic!("Invalid derin attribute: {}", quote!(#attr).to_string())
        }
    }
}

fn expand_generics(generics: &Generics, widget_fields: &[WidgetField]) -> Generics {
    let mut generics = generics.clone();
    generics.ty_params.insert(0, TyParam {
        attrs: Vec::new(),
        ident: Ident::new("__S"),
        bounds: Vec::new(),
        default: None
    });

    let mut init_bound = WhereBoundPredicate {
        bound_lifetimes: Vec::new(),
        bounded_ty: syn::parse_type("__S").unwrap(),
        bounds: vec![TyParamBound::Trait(
            PolyTraitRef {
                bound_lifetimes: Vec::new(),
                trait_ref: syn::parse_path("Sized").unwrap()
            },
            TraitBoundModifier::Maybe
        )]
    };

    for ty in field_types(widget_fields.iter()) {
        let member_bound = WhereBoundPredicate {
            bound_lifetimes: Vec::new(),
            bounded_ty: ty.clone(),
            bounds: vec![TyParamBound::Trait(
                PolyTraitRef{
                    bound_lifetimes: Vec::new(),
                    trait_ref: syn::parse_path(&quote!(_derive_derin::widgets::custom::Widget).to_string()).unwrap(),
                },
                TraitBoundModifier::None
            )]
        };
        generics.where_clause.predicates.push(WherePredicate::BoundPredicate(member_bound));
        init_bound.bounds.push(TyParamBound::Trait(
            PolyTraitRef {
                bound_lifetimes: Vec::new(),
                trait_ref: syn::parse_path(&quote!(_derive_derin::widgets::custom::WidgetSubtype<#ty>).to_string()).unwrap(),
            },
            TraitBoundModifier::None
        ))
    }

    generics.where_clause.predicates.push(WherePredicate::BoundPredicate(init_bound));

    generics
}

fn field_types<'a, I: 'a + Iterator<Item = &'a WidgetField<'a>>>(widget_fields: I) -> impl 'a + Iterator<Item=Ty> {
    widget_fields.map(|widget_field|
        match *widget_field {
            WidgetField::Widget(ref widget_field) => widget_field.ty.clone(),
            WidgetField::Collection(_, ref collection_ty) => collection_ty.clone()
        }
    )
}
