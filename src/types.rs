//! These are our most generic types
//!
//! Based on the concrete Mode our macro might only accept a specific subset of these types.
//! Details are then specified in the documentation of the corresponding modes.

use core::fmt;
use std::num::NonZeroU32;

use crate::modes::forward::FwdInfo;
use crate::modes::reverse::{ReturnActivity, RevInfo};

use super::modes::*;
use syn::parse::{Parse, ParseStream};
use syn::*;
use syn::{Ident, Token};

/// The central Enum being created from macro input.
///
/// Users can't directly create it, so it serves mainly as reference.
/// Please see the documentation of the specific modes to learn how to adjust it's parameters.
#[derive(Clone)]
pub(crate) enum DiffMode {
    Fwd(FwdInfo),
    Rev(RevInfo),
}
impl DiffMode {
    pub(crate) fn name(&self) -> syn::Ident {
        match self {
            DiffMode::Fwd(f) => f.grad_fnc_name.clone(),
            DiffMode::Rev(r) => r.grad_fnc_name.clone(),
        }
    }
    pub(crate) fn ret(&self) -> ReturnActivity {
        match self {
            DiffMode::Fwd(f) => f.return_activity.clone().into(),
            DiffMode::Rev(r) => r.return_activity.clone(),
        }
    }
}
impl fmt::Display for DiffMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DiffMode::Fwd(fwd) => fwd.fmt(f),
            DiffMode::Rev(rev) => rev.fmt(f),
        }
    }
}
impl Parse for DiffMode {
    fn parse(input: ParseStream) -> Result<Self> {
        let grad_fnc_name: Ident = input.parse()?;
        let _: Token![,] = input.parse()?;
        let mode: Mode = input.parse()?;
        let _: Token![,] = input.parse()?;

        match mode {
            Mode::Forward(width) => FwdMode::parse(grad_fnc_name, input, width),
            Mode::Reverse => RevMode::parse(grad_fnc_name, input),
        }
    }
}

pub type Width = NonZeroU32;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Mode {
    /// Forward mode is usually recommendable when having few inputs and various outputs.
    Forward(Width),
    /// Reverse mode is usually recommendable when having various inputs and few outputs.
    Reverse, // None if the fnc returns ()
}
mod kw {
    syn::custom_keyword!(Forward);
    syn::custom_keyword!(Reverse);
}

impl Parse for Mode {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::Reverse) {
            input.parse::<kw::Reverse>()?;
            Ok(Mode::Reverse)
        } else if lookahead.peek(kw::Forward) {
            input.parse::<kw::Forward>()?;
            if lookahead.peek(Token![,]) {
                Ok(Mode::Forward(NonZeroU32::new(1).unwrap()))
            } else {
                let content;
                let _paren_token = parenthesized!(content in input);
                let lit: LitInt = content.parse()?;
                let val = lit.base10_parse::<NonZeroU32>()?;
                Ok(Mode::Forward(val))
            }
        } else {
            Err(lookahead.error())
        }
    }
}
