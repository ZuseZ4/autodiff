//! This crate provides a
#![allow(unused_macros)]
#![doc(html_logo_url = "https://enzyme.mit.edu//logo.svg")]

//use std::default::default;
use std::fmt;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TS2;
use quote::*;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token;
use syn::*;

mod types;
use types::{Activity, Granularity, Mode, ReturnActivity};
mod helper;
use helper::create_ret_struct;

impl Parse for DiffArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let grad_fnc_name: Ident = input.parse()?;
        let _: Token![,] = input.parse()?;
        let mode: Mode = input.parse()?;
        let _: Token![,] = input.parse()?;
        let granularity: Granularity = input.parse()?;
        let _: Token![,] = input.parse()?;
        let ret_activity: ReturnActivity = input.parse()?;
        let _: Token![,] = input.parse()?;
        let parallel_context: LitBool = input.parse()?;
        Ok(DiffArgs {
            grad_fnc_name,
            mode,
            granularity,
            ret_activity,
            parallel_context: parallel_context.value,
        })
    }
}

#[derive(Clone)]
pub(crate) struct DiffArgs {
    grad_fnc_name: Ident,
    mode: types::Mode,
    granularity: types::Granularity,
    ret_activity: ReturnActivity,
    parallel_context: bool,
}

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

/// This one is a preview for the differentiate macro, but adjusted for oxide-enzyme.
/// It will generate and wrap the extern "C" section users had to write previously
/// It's still based on the C-ABI, so all the related issues still apply,
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
fn adjust_parameters(input: DiffArgs, fnc: &mut ForeignItemFn) -> Option<syn::ItemStruct> {
    let out_changes = adjust_input_parameters(input.granularity.clone(), fnc);
    adjust_output_parameters(out_changes, input, fnc)
}

fn make_field(ty: syn::Type, arg_name: String) -> syn::Field {
    Field {
        attrs: vec![],
        vis: Visibility::Inherited,
        ident: Some(Ident::new(&arg_name, proc_macro2::Span::mixed_site())),
        colon_token: Some(Default::default()),
        ty,
    }
}

fn adjust_output_parameters(
    extra_out_params: Vec<syn::Type>,
    infos: DiffArgs,
    fnc: &mut ForeignItemFn,
) -> Option<syn::ItemStruct> {
    // 1. If we don't add return values, we can return early :)
    if extra_out_params.is_empty() {
        match infos.ret_activity {
            ReturnActivity::None | ReturnActivity::Constant => return None,
            ReturnActivity::Ignore => {
                // We also drop the primary return value
                fnc.sig.output = ReturnType::Default;
                return None;
            }
            _ => {} // continue
        };
    }

    // 2. If we add exactly one type (and previously returned () ),
    // then we can return the type directly, without struct around it.
    // Then we also don't have to define a return struct, thus return None.
    if extra_out_params.len() == 1 && infos.ret_activity == ReturnActivity::None {
        fnc.sig.output =
            ReturnType::Type(Default::default(), Box::new(extra_out_params[0].clone()));
        return None;
    }

    // 3. We modify it and end up with multiple types to return,
    // so let's start by creating a new return struct to play with.
    //let mut new_ret_struct = create_ret_struct(infos.grad_fnc_name, fnc.sig.clone());
    let mut new_ret_struct = create_ret_struct(infos.clone(), fnc.sig.clone());

    // 4.a Add the gradient of the primary return, if appropriate
    if infos.ret_activity == ReturnActivity::Active
        || infos.ret_activity == ReturnActivity::Gradient
    {
        let prev_ret = match &fnc.sig.output {
            syn::ReturnType::Default => {
                panic!("Your function returns (), so please don't specify a return activity!");
            }
            syn::ReturnType::Type(_, inner) => *inner.clone(),
        };
        match &mut new_ret_struct.fields {
            syn::Fields::Named(inner) => inner
                .named
                .push(make_field(prev_ret, "primary_grad".to_owned())),
            _ => unreachable!(),
        }
    }

    // 4.b If we have active inputs, add them
    for (arg_num, ret_type) in extra_out_params.iter().enumerate() {
        let extra_ret = ret_type;
        match &mut new_ret_struct.fields {
            syn::Fields::Named(inner) => inner.named.push(make_field(
                extra_ret.clone(),
                "x".to_owned() + &arg_num.to_string(),
            )),
            _ => unreachable!(),
        }
    }

    // 5. Now adjust our function to return the new strucht
    let path_seg: PathSegment = PathSegment {
        ident: new_ret_struct.clone().ident,
        arguments: PathArguments::None,
    };
    let mut segments: Punctuated<PathSegment, Token![::]> = Punctuated::new();
    segments.push(path_seg);
    let path = syn::Path {
        leading_colon: None,
        segments,
    };
    let type_path: syn::TypePath = syn::TypePath { qself: None, path };
    let inner_type: Box<syn::Type> = Box::new(syn::Type::Path(type_path));
    fnc.sig.output = syn::ReturnType::Type(Default::default(), inner_type);

    Some(new_ret_struct)
}

fn make_type(arg: FnArg) -> syn::Type {
    match arg {
        FnArg::Receiver(_) => panic!("self not supported!"),
        FnArg::Typed(pat_ty) => *pat_ty.ty,
    }
}
fn handle_param(
    act: Activity,
    param: syn::FnArg,
    inputs: &mut Punctuated<FnArg, syn::token::Comma>,
    output: &mut Vec<syn::Type>,
) {
    // No matter what, we always keep the primary:
    inputs.push(param.clone());

    // Decide if we add a shaddow to inputs or outputs:
    match act {
        Activity::Active => {
            // Active implies non-ref type
            let ty = make_type(param);
            match ty {
                Type::Ptr(_) | Type::Reference(_) => panic!("Active shall not be used for Pointers or References! Use Gradient or Duplicated."),
                _ => {},
            }
            output.push(ty)
        }
        Activity::Gradient | Activity::Duplicated => {
            // Dup and Gradient require ref type
            if let FnArg::Typed(mut pat_ty) = param {
                match *pat_ty.ty {
                    // We modify the shaddow to make sure it's mutable, 
                    // since we will add the gradients to it.
                    Type::Ptr(ref mut ty_ptr) => {
                        ty_ptr.mutability = Some(Default::default());
                    },
                    Type::Reference(ref mut ty_ref) => {
                        ty_ref.mutability = Some(Default::default());
                    },
                    _ => panic!("Duplicated and Gradient shall only be used for Pointers or References! Use Active instead."),
                }
                inputs.push(FnArg::Typed(pat_ty));
            } else {
                panic!("self not supported!")
            };
        }
        Activity::Constant => {}
    }
}

fn adjust_input_parameters(info: Granularity, fnc: &mut ForeignItemFn) -> Vec<syn::Type> {
    let mut ret_grad_extra_args: Vec<syn::Type> = vec![];
    let params = &mut fnc.sig.inputs;
    let mut new_params: Punctuated<syn::FnArg, syn::token::Comma> = Punctuated::new();
    let mut _arg_num = 0;

    let activities: Vec<Activity> = match info {
        Granularity::All(activity) => vec![activity; params.len()],
        Granularity::PerInput(activities) => {
            assert_eq!(
                params.len(),
                activities.len(),
                "Please provide one activity value per input parameter!"
            );
            activities
        }
    };
    for (&act, param) in activities.iter().zip(params.iter()) {
        handle_param(
            act,
            param.clone(),
            &mut new_params,
            &mut ret_grad_extra_args,
        )
    }
    fnc.sig.inputs = new_params;
    ret_grad_extra_args
}

fn adjust_name(new_name: proc_macro2::Ident, fnc: &mut ForeignItemFn) {
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
