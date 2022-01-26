use super::DiffArgs;
use super::ReturnActivity::*;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::Token;
use syn::*;

// This will be parsed into the TokenStream.
// We need to define a new return struct,
// since tuples are not stable / usable trough the c-abi.
//pub fn create_ret_struct(grad_name: Ident, sig: syn::Signature) -> syn::ItemStruct {
pub(crate) fn create_ret_struct(grad_info: DiffArgs, sig: syn::Signature) -> syn::ItemStruct {
    let grad_name = grad_info.grad_fnc_name;
    let generics = sig.generics;
    let path: syn::Path = syn::Path {
        leading_colon: std::option::Option::None,
        segments: Punctuated::new(),
    };
    let attrs: Vec<syn::Attribute> = vec![syn::Attribute {
        pound_token: Default::default(),
        style: syn::AttrStyle::Outer,
        bracket_token: Default::default(),
        path,
        tokens: quote! {repr(C)},
    }];
    let vis = syn::Visibility::Inherited;
    let struct_token: Token![struct] = Default::default();
    let ident = syn::Ident::new(&(grad_name.to_string() + "_ret"), grad_name.span());
    let mut fields_named = FieldsNamed {
        brace_token: Default::default(),
        named: Punctuated::new(),
    };
    // If our primary function returns something, we might ad that to our return struct
    if let ReturnType::Type(_, box_ty) = sig.output {
        match grad_info.ret_activity {
            None => unreachable!(), // No primary return value exists.
            Gradient | Ignore => {} // The primary return value will be optimized away.
            Active | Constant => {
                // for all other cases, append the primary return value to our return struct
                let primary_id = Ident::new("primary_ret", proc_macro2::Span::mixed_site());
                let field = Field {
                    attrs: vec![],
                    vis: Visibility::Inherited,
                    ident: Some(primary_id),
                    colon_token: Default::default(),
                    ty: *box_ty,
                };
                fields_named.named.push(field);
            }
        }
    }
    let fields = Fields::Named(fields_named);
    let semi_token = std::option::Option::None;
    ItemStruct {
        attrs,
        vis,
        struct_token,
        ident,
        generics,
        fields,
        semi_token,
    }
}
