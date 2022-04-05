//! The reverse-mode Interface

use std::fmt;

use syn::{
    parenthesized, parse::Parse, punctuated::Punctuated, FnArg, ForeignItemFn, Ident, LitBool,
    PathArguments, PathSegment, ReturnType, Token, Type,
};

use crate::{
    helper::create_ret_struct,
    types::{self, DiffMode},
};
use syn::parse::ParseStream;

use super::{make_field, make_type};

#[derive(Clone)]
pub(crate) struct RevInfo {
    pub grad_fnc_name: Ident,
    pub input_activity: Granularity,
    pub return_activity: ReturnActivity,
    pub parallel_context: bool,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum Activity {
    /// The gradient of this input f32/f64 value will be added to the return struct.
    /// The input f32/f64 value will be duplicated, the second parameter will be treated as a
    /// scalar factor.
    Active,
    /// This primal input parameter will be duplicated by adding a shaddow variable.
    /// Enzyme will add \partialf / \partialx to the
    /// shaddow, so you usually want to initialize your shaddow to zero.
    Duplicated,
    /// Similar to Duplicated. However, the primal value will be dropped and can't be used after
    /// calling this function. This might allow extra optimizations in some cases.
    Gradient,
    /// Enzyme will not differente in respect to Constant inputs.
    Constant,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReturnActivity {
    /// return primary ret + gradient
    Active,
    /// return gradient only
    Gradient,
    /// return primary  only
    Constant,
    /// return neither
    Ignore,
    /// primary has no return
    None,
}
#[non_exhaustive]
#[derive(Debug, Clone)]
pub(crate) enum Granularity {
    All(Activity),
    PerInput(Vec<Activity>),
    //PerScalar(..),
}

#[doc(hidden)]
pub(crate) fn parse(
    grad_fnc_name: proc_macro2::Ident,
    input: ParseStream,
) -> Result<DiffMode, syn::Error> {
    let input_activity: Granularity = input.parse()?;
    let _: Token![,] = input.parse()?;
    let return_activity: ReturnActivity = input.parse()?;
    let _: Token![,] = input.parse()?;
    let parallel_context: LitBool = input.parse()?;
    let res = DiffMode::Rev(RevInfo {
        grad_fnc_name,
        input_activity,
        return_activity,
        parallel_context: parallel_context.value,
    });
    Ok(res)
}

#[doc(hidden)]
pub(crate) fn adjust_parameters(
    input: RevInfo,
    fnc: &mut syn::ForeignItemFn,
) -> Option<syn::ItemStruct> {
    let out_changes = adjust_input_parameters(input.input_activity.clone(), fnc);
    adjust_output_parameters(out_changes, input, fnc)
}

#[doc(hidden)]
fn handle_param_rev(
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
        handle_param_rev(
            act,
            param.clone(),
            &mut new_params,
            &mut ret_grad_extra_args,
        )
    }
    fnc.sig.inputs = new_params;
    ret_grad_extra_args
}

#[doc(hidden)]
pub(crate) fn adjust_output_parameters(
    extra_out_params: Vec<syn::Type>,
    infos: RevInfo,
    fnc: &mut ForeignItemFn,
) -> Option<syn::ItemStruct> {
    let ret_act = infos.return_activity;

    // 1. If we don't add return values, we can return early :)
    if extra_out_params.is_empty() {
        match ret_act {
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
    if extra_out_params.len() == 1 && ret_act == ReturnActivity::None {
        fnc.sig.output =
            ReturnType::Type(Default::default(), Box::new(extra_out_params[0].clone()));
        return None;
    }

    // 3. We modify it and end up with multiple types to return,
    // so let's start by creating a new return struct to play with.
    //let mut new_ret_struct = create_ret_struct(infos.grad_fnc_name, fnc.sig.clone());
    let mut new_ret_struct = create_ret_struct(types::DiffMode::Rev(infos), fnc.sig.clone());

    // 4.a Add the gradient of the primary return, if appropriate
    if ret_act == ReturnActivity::Active || ret_act == ReturnActivity::Gradient {
        let prev_ret = match &fnc.sig.output {
            syn::ReturnType::Default => {
                panic!("Your function returns (), so please don't specify a return activity!");
            }
            syn::ReturnType::Type(_, inner) => *inner.clone(),
        };

        if let syn::Fields::Named(ref mut inner) = new_ret_struct.fields {
            let grad_name = "primary_grad".to_owned();
            inner.named.push(make_field(prev_ret, grad_name));
        } else {
            unreachable!();
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

impl fmt::Display for RevInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mode = "reverse-mode";
        let name = self.grad_fnc_name.to_string();
        let par = self.parallel_context;
        let output = format!(
            "handling {name}\nusing {mode}\nwith input activity TODO\nwith output activity TODO\nparallel-context: {par}"
            );
        write!(f, "{output}")
    }
}
impl Parse for Activity {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "Active" => Ok(Activity::Active),
            "Gradient" => Ok(Activity::Gradient),
            "Constant" => Ok(Activity::Constant),
            "Duplicated" => Ok(Activity::Duplicated),
            _ => {
                panic!("Only supporting Active/Gradient/Constant here!")
            }
        }
    }
}

impl Parse for ReturnActivity {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        let out = match ident.to_string().as_str() {
            "Active" => ReturnActivity::Active,
            "Gradient" => ReturnActivity::Gradient,
            "Constant" => ReturnActivity::Constant,
            "Ignore" => ReturnActivity::Ignore,
            "None" => ReturnActivity::None,
            _ => panic!("Failed parsing return activity. Please specify None if you return () and an activity otherwise!")
        };
        Ok(out)
    }
}

impl Parse for Granularity {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let category: Ident = input.parse()?;

        let content;
        let _paren_token = parenthesized!(content in input);
        let activities: Punctuated<Activity, Token![,]> =
            content.parse_terminated(Activity::parse)?;
        let activities: Vec<Activity> = activities.into_iter().collect();
        match category.to_string().as_str() {
            "All" => {
                assert_eq!(activities.len(), 1);
                Ok(Granularity::All(activities[0]))
            }
            "PerInput" => Ok(Granularity::PerInput(activities)),
            _ => unimplemented!("Expected All or PerInput. Got {}", category),
        }
    }
}
