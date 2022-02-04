//! These are our most generic types
//!
//! Based on the concrete Mode our macro might only accept a specific subset of these types.
//! Details are then specified in the documentation of the corresponding modes.

use super::modes::*;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;
use syn::{Ident, Token};

/// The central struct being created from macro input.
///
/// Users can't directly create it, so it serves mainly as reference.
/// Please see the documentation of the specific modes to learn how to adjust it's parameters.
#[derive(Clone)]
pub(crate) struct DiffArgs {
    pub grad_fnc_name: Ident,
    pub mode: Mode,
    pub granularity: Granularity,
    pub ret_activity: ReturnActivity,
    pub parallel_context: bool,
}
impl Parse for DiffArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let grad_fnc_name: Ident = input.parse()?;
        let _: Token![,] = input.parse()?;
        let mode: Mode = input.parse()?;
        let _: Token![,] = input.parse()?;

        match mode {
            Mode::Forward => FwdMode::parse(grad_fnc_name, input),
            Mode::Reverse => RevMode::parse(grad_fnc_name, input),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Activity {
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
impl Parse for Activity {
    fn parse(input: ParseStream) -> Result<Self> {
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

impl Parse for ReturnActivity {
    fn parse(input: ParseStream) -> Result<Self> {
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

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Granularity {
    All(Activity),
    PerInput(Vec<Activity>),
    //PerScalar(..),
}
impl Parse for Granularity {
    fn parse(input: ParseStream) -> Result<Self> {
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

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Mode {
    /// Forward mode is usually recommendable when having few inputs and various outputs.
    Forward,
    /// Reverse mode is usually recommendable when having various inputs and few outputs.
    Reverse, // None if the fnc returns ()
}

impl Parse for Mode {
    fn parse(input: ParseStream) -> Result<Self> {
        let mode: Ident = input.parse()?;

        // (maybe?) TODO: replace unimplemented with syn::error
        let mode_str = mode.to_string();
        let res = match mode_str.as_str() {
            "Forward" => Mode::Forward,
            "Reverse" => Mode::Reverse,
            _ => unimplemented!("Expected forward or Reverse. got {}", mode_str),
        };
        Ok(res)
    }
}
