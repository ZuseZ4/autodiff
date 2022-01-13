#![allow(unused_macros)]
#![doc(html_logo_url = "https://enzyme.mit.edu//logo.svg")]

use std::fmt;

use syn::__private::ToTokens;

use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::*;

mod types;
use types::{Activity, Granularity, Mode};

impl Parse for DiffStruct {
    fn parse(input: ParseStream) -> Result<Self> {
        let primary_fnc: TypePath = input.parse()?;
        let _: Token![,] = input.parse()?;
        let mode: Mode = input.parse()?;
        let _: Token![,] = input.parse()?;
        let granularity: Granularity = input.parse()?;
        let _: Token![,] = input.parse()?;
        let parallel_context: LitBool = input.parse()?;
        Ok(DiffStruct {
            primary_fnc: primary_fnc
                .path
                .to_token_stream()
                .to_string()
                .replace(" ", ""),
            mode,
            granularity,
            parallel_context: parallel_context.value,
        })
    }
}

struct DiffStruct {
    primary_fnc: String,
    mode: types::Mode,
    granularity: types::Granularity,
    parallel_context: bool,
}

struct PrimaryFnc {
    // TODO better ItemFn
}

impl fmt::Display for DiffStruct {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "primary: {} \nmode: {:?} \ngranularity: {:?} \nparallel_context: {} \n",
            self.primary_fnc, self.mode, self.granularity, self.parallel_context
        )
    }
}

fn check(macro_args: DiffStruct, fnc: ItemFn) {
    unimplemented!()
}

#[proc_macro_attribute]
pub fn register_derivative(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: DiffStruct = parse_macro_input!(attr as DiffStruct);
    let out = item.clone();
    let fnc: ItemFn = parse_macro_input!(item as ItemFn);
    println!("{}", input);
    println!("{:?}", fnc);
    println!("\n");
    check(input, fnc);
    out
}

#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn differentiate(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
