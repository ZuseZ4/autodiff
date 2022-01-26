//! This is a test

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;
use syn::{Ident, Token};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Activity {
    Active,
    Gradient,
    Duplicated,
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
    Active,   // return primary ret + gradient
    Gradient, // return gradient only
    Constant, // return primary  only
    Ignore,   // return neither
    None,     // primary has no return
}
// pub struct ReturnActivity {
//     inner: Option<Activity>,
// }

/*
impl ReturnActivity {
pub fn deconstruct(self) -> Option<Activity> {
self.inner
}
fn new(inner: Option<Activity>) -> Self {
ReturnActivity { inner }
}
}
*/
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

#[doc = "Mode"]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Mode {
    Forward,
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
