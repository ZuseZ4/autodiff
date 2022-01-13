//! This is a test

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;
use syn::{Ident, Token};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Activity {
    Active,
    Gradient,
    Constant,
}
impl Parse for Activity {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "Active" => Ok(Activity::Active),
            "Gradient" => Ok(Activity::Gradient),
            "Constant" => Ok(Activity::Constant),
            _ => {
                panic!("Only supporting Active/Gradient/Constant here!")
            }
        }
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
                assert_ne!(
                    activities[0],
                    Activity::Constant,
                    "Setting all inputs as constant doesn't make sense"
                ); // TODO: verify, what if output active?
                Ok(Granularity::All(activities[0]))
            }
            "PerInput" => Ok(Granularity::PerInput(activities)),
            _ => unimplemented!("Expected All or PerInput. Got {}", category),
        }
    }
}

#[doc = "Mode"]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Mode {
    Forward,
    Reverse(Option<Activity>), // None if the fnc returns ()
}

impl Parse for Mode {
    fn parse(input: ParseStream) -> Result<Self> {
        let mode: Ident = input.parse()?;

        let inner: Option<Activity> = if input.peek(Token![,]) {
            None
        } else {
            let content;
            let _paren_token = parenthesized!(content in input);
            let activity: Activity = content.parse()?;
            Some(activity)
        };
        // (maybe?) TODO: replace unimplemented with syn::error
        let mode_str = mode.to_string();
        let res = match mode_str.as_str() {
            "Forward" => Mode::Forward,
            "Reverse" => Mode::Reverse(inner),
            _ => unimplemented!("Expected forward or Reverse. got {}", mode_str),
        };
        Ok(res)
    }
}
