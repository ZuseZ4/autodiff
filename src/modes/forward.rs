//! The forward-mode Interface
//!
//! It is
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{parenthesized, Ident};
use syn::{parse::ParseStream, LitBool, Token};

use crate::types::{Activity, DiffArgs, Granularity, Mode, ReturnActivity, Width};

use super::default;

#[doc(hidden)]
pub(crate) fn adjust_parameters(
    input: DiffArgs,
    fnc: &mut syn::ForeignItemFn,
) -> Option<syn::ItemStruct> {
    let out_changes = default::adjust_input_parameters(input.granularity.clone(), fnc);
    default::adjust_output_parameters(out_changes, input, fnc)
}

// Re-implementation (I guess due to missing Specification)
// to only allow Const / Duplicated as (Return)Activity
// Should give better user error messages compared to later catching.

/// Forward-Mode uses a more restricted version of the general Activity enum.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FwdActivity {
    /// We expect
    Duplicated,
    Constant,
}
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
impl From<FwdActivity> for Activity {
    fn from(input: FwdActivity) -> Self {
        match input {
            FwdActivity::Constant => Activity::Constant,
            FwdActivity::Duplicated => Activity::Duplicated,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FwdReturnActivity {
    Active,   // return primary ret + gradient
    Gradient, // return gradient only
    Constant, // return primary only
    None,     // primary has no return
}

impl Parse for FwdReturnActivity {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let ident: Ident = input.parse()?;
        let out = match ident.to_string().as_str() {
            "Active" => FwdReturnActivity::Active,
            "Gradient" => FwdReturnActivity::Gradient,
            "Constant" => FwdReturnActivity::Constant,
            "None" => FwdReturnActivity::None,
            _ => panic!("Failed parsing return activity. Please specify None if you return () and an activity otherwise!")
        };
        Ok(out)
    }
}
impl From<FwdReturnActivity> for ReturnActivity {
    fn from(input: FwdReturnActivity) -> Self {
        match input {
            FwdReturnActivity::Active => ReturnActivity::Active,
            FwdReturnActivity::Gradient => ReturnActivity::Gradient,
            FwdReturnActivity::Constant => ReturnActivity::Constant,
            FwdReturnActivity::None => ReturnActivity::None,
        }
    }
}

#[doc(hidden)]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum FwdGranularity {
    All(FwdActivity),
    PerInput(Vec<FwdActivity>),
    //PerScalar(..),
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
impl From<FwdGranularity> for Granularity {
    fn from(input: FwdGranularity) -> Self {
        match input {
            FwdGranularity::All(activity) => Granularity::All(activity.into()),
            FwdGranularity::PerInput(activity_vec) => Granularity::PerInput(
                activity_vec
                    .iter()
                    .map(|&a| a.into())
                    .collect::<Vec<Activity>>(),
            ),
        }
    }
}

#[doc(hidden)]
pub(crate) fn parse(
    grad_fnc_name: proc_macro2::Ident,
    input: ParseStream,
    width: Width,
) -> Result<DiffArgs, syn::Error> {
    let granularity: FwdGranularity = input.parse()?;
    let _: Token![,] = input.parse()?;
    let ret_activity: FwdReturnActivity = input.parse()?;
    let _: Token![,] = input.parse()?;
    let parallel_context: LitBool = input.parse()?;
    Ok(DiffArgs {
        grad_fnc_name,
        mode: Mode::Forward(width),
        granularity: granularity.into(),
        ret_activity: ret_activity.into(),
        parallel_context: parallel_context.value,
    })
}
