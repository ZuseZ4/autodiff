use syn::{
    punctuated::Punctuated, FnArg, ForeignItemFn, PathArguments, PathSegment, ReturnType, Token,
    Type,
};

use super::make_field;
use super::make_type;
use crate::{
    helper::create_ret_struct,
    types::{Activity, DiffArgs, Granularity, ReturnActivity},
};

#[doc(hidden)]
pub(crate) fn adjust_output_parameters(
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
#[doc(hidden)]
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
            // Used as linear factor
            inputs.push(param.clone());

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

#[doc(hidden)]
pub(crate) fn adjust_input_parameters(
    info: Granularity,
    fnc: &mut ForeignItemFn,
) -> Vec<syn::Type> {
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
