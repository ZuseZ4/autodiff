//! This crate provides a convenient interface to Enzyme (and later hopefully additional AD tools).
//!
//! The core is our differentiate attribute-proc-macro.  
//! The parameters which it accepts might can differ slightly depending on the mode which you select.  
//! This is how it will generaly look like.  
//! `#[differentiate(grad_fnc_name, mode, activity_inputs, activity_output, parallel_context)]`

#![allow(unused_macros)]
#![doc(html_logo_url = "https://enzyme.mit.edu//logo.svg")]

use std::fmt;

use modes::{forward, reverse};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TS2;
use quote::*;
use syn::punctuated::Punctuated;
use syn::token;
use syn::*;

mod types;
use types::{DiffArgs, Mode, ReturnActivity};
#[doc(hidden)]
mod helper;
mod modes;
// use modes::{FwdMode, RevMode};

#[doc(hidden)]
// only append no_mangle if not already in there
fn append_no_mangle(item: &mut ItemFn) {
    let nomangle_attr = Attribute {
        pound_token: Default::default(),
        style: AttrStyle::Outer,
        bracket_token: Default::default(),
        path: Path {
            leading_colon: None,
            segments: Punctuated::new(),
        },
        tokens: quote! { no_mangle },
    };

    // don't add it multiple times
    for attr in item.attrs.iter() {
        let attr_ps = &attr.path.segments;
        if attr_ps.len() == 1 {
            // no_mangle attr has one segment
            let id = attr_ps[0].ident.to_string();
            if id == *"no_mangle".to_owned() {
                return;
            }
        }
    }
    item.attrs.push(nomangle_attr);
}

/// Thisis a preview for a generic differentiate macro, adjusted for oxide-enzyme.  
///
/// It will generate and wrap the extern "C" section users had to write previously.  
/// It is still based on the C-ABI, so all the related issues still apply,
/// but at least it's nicer to use.
#[proc_macro_attribute]
pub fn differentiate_ext(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: DiffArgs = parse_macro_input!(attr as DiffArgs);
    let mut primary_fnc: ItemFn = parse_macro_input!(item as ItemFn);
    append_no_mangle(&mut primary_fnc);
    let mut fnc = ForeignItemFn {
        semi_token: token::Semi::default(),
        attrs: vec![],
        vis: primary_fnc.vis.clone(),
        sig: primary_fnc.sig.clone(),
    };
    let mut out = primary_fnc.to_token_stream();
    adjust_name(input.grad_fnc_name.clone(), &mut fnc);
    let ret_struct_def: Option<syn::ItemStruct> = adjust_parameters(input, &mut fnc);
    let ext_block: TS2 = quote! {
        extern "C" { #fnc }
    };

    out.extend(ext_block);
    if let Some(struct_def) = ret_struct_def {
        out.extend(struct_def.to_token_stream());
    }
    out.into()
}

#[doc(hidden)]
fn adjust_name(new_name: syn::Ident, fnc: &mut ForeignItemFn) {
    assert_ne!(
        fnc.sig.ident, new_name,
        "Please give the gradient function to be generated a new name!"
    );
    fnc.sig.ident = new_name;
}

impl fmt::Display for DiffArgs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "primary: {} \nmode: {:?} \ngranularity: {:?} \nparallel_context: {} \n",
            self.grad_fnc_name, self.mode, self.granularity, self.parallel_context
        )
    }
}

#[doc(hidden)]
pub(crate) fn adjust_parameters(
    input: DiffArgs,
    fnc: &mut ForeignItemFn,
) -> Option<syn::ItemStruct> {
    match input.mode {
        Mode::Reverse => reverse::adjust_parameters(input, fnc),
        Mode::Forward => forward::adjust_parameters(input, fnc),
    }
}
