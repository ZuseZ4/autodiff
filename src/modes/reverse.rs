//! The reverse-mode Interface

use syn::{LitBool, Token};

use crate::types::{DiffArgs, Granularity, Mode, ReturnActivity};
use syn::parse::ParseStream;

use super::default;

#[doc(hidden)]
pub(crate) fn parse(
    grad_fnc_name: proc_macro2::Ident,
    input: ParseStream,
) -> Result<DiffArgs, syn::Error> {
    let granularity: Granularity = input.parse()?;
    let _: Token![,] = input.parse()?;
    let ret_activity: ReturnActivity = input.parse()?;
    let _: Token![,] = input.parse()?;
    let parallel_context: LitBool = input.parse()?;
    Ok(DiffArgs {
        grad_fnc_name,
        mode: Mode::Reverse,
        granularity,
        ret_activity,
        parallel_context: parallel_context.value,
    })
}

#[doc(hidden)]
pub(crate) fn adjust_parameters(
    input: DiffArgs,
    fnc: &mut syn::ForeignItemFn,
) -> Option<syn::ItemStruct> {
    let out_changes = default::adjust_input_parameters(input.granularity.clone(), fnc);
    default::adjust_output_parameters(out_changes, input, fnc)
}
