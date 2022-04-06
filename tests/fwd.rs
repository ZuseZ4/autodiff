#![allow(unused)]
#![allow(non_camel_case_types)]

use autodiff::differentiate_ext;

// #[differentiate_ext(d_fwd_f, Forward, All(Duplicated), Active, false)]
// #[differentiate_ext(d_fwd_f1, Forward, All(Duplicated), Gradient, false)]
// #[differentiate_ext(d_fwd_f2, Forward(4), All(Duplicated), Active, false)]
// fn f(x: f32) -> f32 {
//     2.0 * x
// }

#[repr(C)]
#[derive(Debug, Clone)]
struct ret_tuple {
    a: f32,
    b: f32,
}

#[differentiate_ext(d_fwd_g, Forward, All(Duplicated), Active, false)]
#[differentiate_ext(d_fwd_g1, Forward, All(Duplicated), Gradient, false)]
#[differentiate_ext(d_fwd_g2, Forward(4), All(Duplicated), Active, false)]
fn g(x: f32, y: f32) -> ret_tuple {
    ret_tuple {
        a: 2.0 * x,
        b: y * x,
    }
}
