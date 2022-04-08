//! The forward-mode Interface
//!
//! It is
use std::fmt;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{parenthesized, FnArg, ForeignItemFn, Ident};
use syn::{parse::ParseStream, Token};

use crate::helper::create_ret_struct;
use crate::types::{self, DiffMode, Width};

use super::make_field;
use super::reverse::ReturnActivity;

//
// Here we define some types relevant for forward-mode AD
//

/// Forward-Mode uses a more restricted version of the general Activity enum.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FwdActivity {
    /// We expect
    Duplicated,
    Constant,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FwdReturnActivity {
    Active,   // return primary ret + gradient
    Gradient, // return gradient only
}
#[doc(hidden)]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum FwdGranularity {
    All(FwdActivity),
    PerInput(Vec<FwdActivity>),
    //PerScalar(..),
}
#[derive(Clone)]
pub(crate) struct FwdInfo {
    pub grad_fnc_name: Ident,
    pub width: Width,
    pub input_activity: FwdGranularity,
    pub return_activity: FwdReturnActivity,
}

//
// Here we define the key functions to generate the declaration of our derivative function,
// as well as the struct it will return.
//

#[doc(hidden)]
pub(crate) fn adjust_parameters(
    infos: FwdInfo,
    fnc: &mut syn::ForeignItemFn,
) -> Option<syn::ItemStruct> {
    // First, we need to create <width> copies of each active input

    let params = &mut fnc.sig.inputs;
    let mut new_params: Punctuated<syn::FnArg, syn::token::Comma> = Punctuated::new();

    let activities: Vec<FwdActivity> = match infos.input_activity {
        FwdGranularity::All(activity) => vec![activity; params.len()],
        FwdGranularity::PerInput(ref activities) => {
            assert_eq!(
                params.len(),
                activities.len(),
                "Please provide one activity value per input parameter!"
            );
            activities.clone()
        }
    };

    for (&act, param) in activities.iter().zip(params.iter()) {
        handle_input_params_fwd(infos.width, act, param.clone(), &mut new_params)
    }
    fnc.sig.inputs = new_params;

    // Second, we need to create <width> copies of the output and (optionally) use the primary output too.
    adjust_output_parameters(infos, fnc)
}

#[doc(hidden)]
pub(crate) fn adjust_output_parameters(
    infos: FwdInfo,
    fnc: &mut ForeignItemFn,
) -> Option<syn::ItemStruct> {
    let ret_act = infos.return_activity;
    let mut new_ret_struct =
        create_ret_struct(types::DiffMode::Fwd(infos.clone()), fnc.sig.clone());
    // 4.a Add the gradient of the primary return, if appropriate
    if ret_act == FwdReturnActivity::Active || ret_act == FwdReturnActivity::Gradient {
        let prev_ret = match &fnc.sig.output {
            syn::ReturnType::Default => {
                panic!("Your function returns (), so please don't specify a return activity!");
            }
            syn::ReturnType::Type(_, inner) => *inner.clone(),
        };

        if let syn::Fields::Named(ref mut inner) = new_ret_struct.fields {
            let grad_name = "primary_grad".to_owned();
            let width_u32 = u32::from(infos.width);
            if width_u32 == 1 {
                inner.named.push(make_field(prev_ret, grad_name));
            } else {
                // Forward-Mode-Vector
                for i in 0..width_u32 {
                    inner.named.push(make_field(
                        prev_ret.clone(),
                        grad_name.clone() + &i.to_string(),
                    ));
                }
            }
        } else {
            unreachable!();
        }
    }
    Some(new_ret_struct)
}

#[doc(hidden)]
fn handle_input_params_fwd(
    width: Width,
    act: FwdActivity,
    param: syn::FnArg,
    inputs: &mut Punctuated<FnArg, syn::token::Comma>,
) {
    // No matter what, we always keep the primary:
    inputs.push(param.clone());

    if let FwdActivity::Constant = act {
        return; // We don't duplicate constant inputs
    } // else is always FwdActivity::Duplicated

    let u32_width = u32::from(width);
    let usize_width: usize = u32_width.try_into().unwrap();
    let params = vec![param; usize_width];

    for (i, mut param) in params.into_iter().enumerate() {
        // There is no reasonable way to differentiate methods containing self.
        let pat_ty = match param {
            FnArg::Typed(ref mut pat_ty) => pat_ty,
            FnArg::Receiver(_) => panic!("self not supported!"),
        };

        // Unlike in the reverse pass, we won't modify inputs during runtime.
        // So we don't require mutability of inputs here.

        if let syn::Pat::Ident(ref mut pat_ident) = *pat_ty.pat {
            let mut base_name = pat_ident.ident.to_string();
            base_name = format!("d_{base_name}");
            let pat_span = pat_ident.ident.span();

            let input_name = format!("{base_name}_{i}");
            pat_ident.ident = syn::Ident::new(&input_name, pat_span);
            inputs.push(param);
        } else {
            unreachable!("implementation error")
        }
    }
}

// Re-implementation (I guess due to missing Specification)
// to only allow Const / Duplicated as (Return)Activity
// Should give better user error messages compared to later catching.

impl Parse for FwdActivity {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "Constant" => Ok(FwdActivity::Constant),
            "Duplicated" => Ok(FwdActivity::Duplicated),
            _ => {
                panic!("Forward Mode AD only supports Duplicated and Constant here!")
            }
        }
    }
}

impl From<FwdReturnActivity> for ReturnActivity {
    fn from(f: FwdReturnActivity) -> Self {
        match f {
            FwdReturnActivity::Active => ReturnActivity::Active,
            FwdReturnActivity::Gradient => ReturnActivity::Gradient,
        }
    }
}

impl Parse for FwdReturnActivity {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let ident: Ident = input.parse()?;
        let out = match ident.to_string().as_str() {
            "Active" => FwdReturnActivity::Active,
            "Gradient" => FwdReturnActivity::Gradient,
            _ => panic!("Failed parsing return activity. Please use Active or Gradient!"),
        };
        Ok(out)
    }
}

impl Parse for FwdGranularity {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let category: Ident = input.parse()?;

        let content;
        let _paren_token = parenthesized!(content in input);
        let activities: Punctuated<FwdActivity, Token![,]> =
            content.parse_terminated(FwdActivity::parse)?;
        let activities: Vec<FwdActivity> = activities.into_iter().collect();
        match category.to_string().as_str() {
            "All" => {
                assert_eq!(activities.len(), 1);
                Ok(FwdGranularity::All(activities[0]))
            }
            "PerInput" => Ok(FwdGranularity::PerInput(activities)),
            _ => unimplemented!("Expected All or PerInput. Got {}", category),
        }
    }
}

impl fmt::Display for FwdInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let u32_width = u32::from(self.width);
        let mode = if u32_width > 1 {
            "fwd-mode-vector({u32_width})"
        } else {
            "fwd-mode"
        };
        let name = self.grad_fnc_name.to_string();
        let output = format!(
            "handling {name}\nusing {mode}\nwith input activity TODO\nwith output activity TODO"
        );
        write!(f, "{output}")
    }
}

#[doc(hidden)]
pub(crate) fn parse(
    grad_fnc_name: proc_macro2::Ident,
    input: ParseStream,
    width: Width,
) -> Result<DiffMode, syn::Error> {
    let granularity: FwdGranularity = input.parse()?;
    let _: Token![,] = input.parse()?;
    let return_activity: FwdReturnActivity = input.parse()?;

    let res = types::DiffMode::Fwd(FwdInfo {
        grad_fnc_name,
        width,
        input_activity: granularity,
        return_activity,
    });
    Ok(res)
}
