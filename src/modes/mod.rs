//! An overview of the AD modes which we support
#[doc(hidden)]
pub mod forward;
pub mod reverse;

#[doc(hidden)]
pub use forward as FwdMode;
#[doc(hidden)]
pub use reverse as RevMode;
use syn::{Field, FnArg, Ident, Visibility};

#[doc(hidden)]
fn make_field(ty: syn::Type, arg_name: String) -> syn::Field {
    Field {
        attrs: vec![],
        vis: Visibility::Inherited,
        ident: Some(Ident::new(&arg_name, proc_macro2::Span::mixed_site())),
        colon_token: Some(Default::default()),
        ty,
    }
}

#[doc(hidden)]
fn make_type(arg: FnArg) -> syn::Type {
    match arg {
        FnArg::Receiver(_) => panic!("self not supported!"),
        FnArg::Typed(pat_ty) => *pat_ty.ty,
    }
}
