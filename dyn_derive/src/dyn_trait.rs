use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};

use crate::generics::GenericsData;
use crate::subst::Context;

fn supertraits(fact: &mut syn::ItemTrait, inst: &mut syn::ItemTrait, cons: &mut syn::ItemTrait) -> TokenStream {
    let mut has_sized = false;
    let inst_ident = &inst.ident;
    let (impl_generics, type_generics, where_clause) = inst.generics.split_for_impl();
    let mut output = quote! {};
    inst.supertraits = syn::punctuated::Punctuated::from_iter(fact.supertraits.iter_mut().flat_map(|param| {
        let syn::TypeParamBound::Trait(fact_bound) = param else {
            return Some(param.clone())
        };
        let mut inst_bound = fact_bound.clone();
        let op = inst_bound.path.to_token_stream().to_string();
        match op.as_str() {
            "Sized" => {
                has_sized = true;
                return None
            },
            "Clone" => {
                inst_bound.path = syn::parse_quote! { ::dyn_std::clone::Clone };
                output.extend(quote! {
                    #[automatically_derived]
                    impl #impl_generics Clone for Box<dyn #inst_ident #type_generics> #where_clause {
                        #[inline]
                        fn clone(&self) -> Self {
                            ::dyn_std::Fat::to_box(self, ::dyn_std::clone::Clone::dyn_clone)
                        }
                    }
                });
            },
            "PartialEq" | "PartialOrd" => {
                let name = format_ident!("{}", op);
                let (method, dyn_method, return_type) = match op.as_str() {
                    "PartialEq" => (quote!(eq), quote!(dyn_eq), quote!(bool)),
                    "PartialOrd" => (quote!(partial_cmp), quote!(dyn_partial_cmp), quote!(Option<std::cmp::Ordering>)),
                    _ => unreachable!(),
                };
                inst_bound.path = syn::parse_quote! { ::dyn_std::cmp::#name };
                output.extend(quote! {
                    #[automatically_derived]
                    impl #impl_generics std::cmp::#name for dyn #inst_ident #type_generics #where_clause {
                        #[inline]
                        fn #method(&self, other: &Self) -> #return_type {
                            self.#dyn_method(other.as_any())
                        }
                    }
                });
                // Workaround Rust compiler bug:
                // https://github.com/rust-lang/rust/issues/31740#issuecomment-700950186
                output.extend(quote! {
                    #[automatically_derived]
                    impl #impl_generics std::cmp::#name<&Self> for Box<dyn #inst_ident #type_generics> #where_clause {
                        #[inline]
                        fn #method(&self, other: &&Self) -> #return_type {
                            self.#dyn_method(other.as_any())
                        }
                    }
                });
            },
            "Neg" | "Not" => {
                let name = format_ident!("{}", op);
                let method = format_ident!("{}", op.to_lowercase());
                let dyn_method = format_ident!("dyn_{}", method);
                inst_bound.path = syn::parse_quote! { ::dyn_std::ops::#name };
                fact_bound.path = syn::parse_quote! { #name<Output = Self> };
                output.extend(quote! {
                    #[automatically_derived]
                    impl #impl_generics std::ops::#name for Box<dyn #inst_ident #type_generics> #where_clause {
                        type Output = Self;
                        #[inline]
                        fn #method(self) -> Self {
                            ::dyn_std::Fat::into_box(self, |m| m.#dyn_method())
                        }
                    }
                });
            },
            "Add" | "Sub" | "Mul" | "Div" | "Rem" |
            "BitAnd" | "BitOr" | "BitXor" | "Shl" | "Shr" => {
                let name = format_ident!("{}", op);
                let method = format_ident!("{}", op.to_lowercase());
                let dyn_method = format_ident!("dyn_{}", method);
                inst_bound.path = syn::parse_quote! { ::dyn_std::ops::#name };
                fact_bound.path = syn::parse_quote! { #name<Output = Self> };
                output.extend(quote! {
                    #[automatically_derived]
                    impl #impl_generics std::ops::#name for Box<dyn #inst_ident #type_generics> #where_clause {
                        type Output = Self;
                        #[inline]
                        fn #method(self, other: Self) -> Self {
                            ::dyn_std::Fat::into_box(self, |m| m.#dyn_method(other.as_any_box()))
                        }
                    }
                });
            },
            "AddAssign" | "SubAssign" | "MulAssign" | "DivAssign" | "RemAssign" |
            "BitAndAssign" | "BitOrAssign" | "BitXorAssign" | "ShlAssign" | "ShrAssign" => {
                let name = format_ident!("{}", op);
                let method = format_ident!("{}_assign", op[0..op.len() - 6].to_lowercase());
                let dyn_method = format_ident!("dyn_{}_assign", method);
                inst_bound.path = syn::parse_quote! { ::dyn_std::ops::#name };
                output.extend(quote! {
                    #[automatically_derived]
                    impl #impl_generics std::ops::#name for Box<dyn #inst_ident #type_generics> #where_clause {
                        #[inline]
                        fn #method(&mut self, other: Self) {
                            self.#dyn_method(other.as_any_box())
                        }
                    }
                });
            },
            _ => {},
        }
        Some(syn::TypeParamBound::Trait(inst_bound))
    }));
    if !has_sized {
        fact.supertraits.push(syn::parse_quote! { Sized });
    }
    fact.supertraits.push(syn::parse_quote! { 'static });
    inst.supertraits.push(syn::parse_quote! { ::dyn_std::any::Dyn });
    cons.supertraits = Default::default();
    output
}

pub fn get_full_name(item: &syn::ItemTrait) -> (TokenStream, TokenStream) {
    let ident = &item.ident;
    let mut generic_params = vec![];
    let mut instance_params = vec![];
    for param in &item.generics.params {
        match param {
            syn::GenericParam::Type(param) => {
                let ident = &param.ident;
                generic_params.push(quote! { #ident });
                instance_params.push(quote! { #ident });
            },
            syn::GenericParam::Lifetime(param) => {
                let lifetime = &param.lifetime;
                generic_params.push(quote! { #lifetime });
            },
            syn::GenericParam::Const(_) => {
                unimplemented!("const generics in traits")
            },
        }
    }
    (
        match generic_params.len() {
            0 => quote! { #ident },
            _ => quote! { #ident<#(#generic_params),*> },
        },
        match instance_params.len() {
            0 => quote! { () },
            _ => quote! { (#(#instance_params),*,) },
        },
    )
}

pub fn transform(_attr: TokenStream, mut fact: syn::ItemTrait) -> TokenStream {
    let mut inst = fact.clone();
    inst.ident = format_ident!("{}Instance", fact.ident);
    inst.items = Default::default();
    let (inst_trait, _) = get_full_name(&inst);
    let generics = GenericsData::from(inst_trait.clone(), &mut fact);
    let mut cons = inst.clone();
    cons.ident = format_ident!("{}Constructor", fact.ident);
    let (cons_trait, _) = get_full_name(&cons);
    let super_impls = supertraits(&mut fact, &mut inst, &mut cons);
    for param in fact.generics.params.iter_mut() {
        let syn::GenericParam::Type(param) = param else {
            continue;
        };
        let index = param.attrs.iter().position(|attr| {
            attr.meta.path().is_ident("dynamic")
        });
        if let Some(index) = index {
            param.attrs.remove(index);
        } else {
            param.bounds.push(syn::parse_quote! { 'static });
            continue;
        }
    }
    let (fact_trait, _) = get_full_name(&fact);
    let mut inst_impl_items = vec![];
    let mut cons_impl_items = vec![];
    for fact_item in &generics.items {
        match fact_item {
            syn::TraitItem::Fn(item_fn) => {
                let mut item_fn = item_fn.clone();
                let ident = &item_fn.sig.ident;
                let has_recv = item_fn.sig.receiver().is_some();
                if !has_recv {
                    item_fn.sig.inputs.insert(0, syn::parse_quote! { &self });
                }
                let ctx = Context::new(&generics);
                let inputs = item_fn.sig.inputs.iter_mut().filter_map(|arg| {
                    match arg {
                        syn::FnArg::Typed(arg) => Some(arg.ty.as_mut()),
                        syn::FnArg::Receiver(recv) => {
                            if recv.ty.to_token_stream().to_string() == "Self" {
                                recv.ty = syn::parse_quote! { Box<Self> };
                            }
                            None
                        },
                    }
                });
                let (expr, stmts, params, _) = ctx.subst_fn(inputs, &mut item_fn.sig.output, &match has_recv {
                    true => quote! { self.0.#ident },
                    false => quote! { Factory::#ident },
                });
                let mut impl_fn = syn::ImplItemFn {
                    attrs: vec![syn::parse_quote! { #[inline] }],
                    vis: syn::Visibility::Inherited,
                    defaultness: None,
                    sig: item_fn.sig.clone(),
                    block: syn::parse_quote! {{ #stmts #expr }},
                };
                impl_fn.sig.inputs
                    .iter_mut()
                    .filter_map(|arg| {
                        match arg {
                            syn::FnArg::Typed(arg) => Some(arg),
                            syn::FnArg::Receiver(_) => None,
                        }
                    })
                    .zip(params.into_iter())
                    .for_each(|(arg, pat)| {
                        arg.pat = Box::new(syn::parse_quote! { #pat });
                    });
                if has_recv {
                    inst.items.push(syn::TraitItem::Fn(item_fn));
                    inst_impl_items.push(impl_fn);
                } else {
                    cons.items.push(syn::TraitItem::Fn(item_fn));
                    cons_impl_items.push(impl_fn);
                }
            },
            _ => {
                inst.items.push(fact_item.clone());
                cons.items.push(fact_item.clone());
            },
        }
    }
    let mut fact_generics = fact.generics.clone();
    fact_generics.params.push(syn::parse_quote! { Factory: #fact_trait });
    let (impl_generics, _, where_clause) = fact_generics.split_for_impl();
    quote! {
        #fact
        #inst
        #cons
        #super_impls
        #[automatically_derived]
        impl #impl_generics #inst_trait for ::dyn_std::Instance<Factory> #where_clause {
            #(#inst_impl_items)*
        }
        #[automatically_derived]
        impl #impl_generics #cons_trait for ::dyn_std::Constructor<Factory> #where_clause {
            #(#cons_impl_items)*
        }
    }
}
