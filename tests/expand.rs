#![allow(unused)]
#![allow(non_camel_case_types)]

use autodiff::differentiate_ext;

#[differentiate_ext(d_a2, Reverse, PerInput(Constant, Active), None, false)]
#[differentiate_ext(d_a3, Reverse, PerInput(Gradient, Constant), None, false)]
#[differentiate_ext(d_a1, Reverse, PerInput(Gradient, Active), None, false)]
fn a(x: &mut f32, y: f32) {
    *x *= y
}

#[differentiate_ext(d_b, Reverse, All(Gradient), None, false)]
fn b(x: &mut f32) {
    *x *= 2.0
}

#[differentiate_ext(d_c, Reverse, All(Gradient), Constant, false)]
fn c(x: &mut f32) -> f32 {
    *x * 2.0
}

#[differentiate_ext(d_d1, Reverse, All(Active), Gradient, false)]
fn d1(x: f32) -> f32 {
    2.0 * x
}
#[differentiate_ext(d_d, Reverse, All(Active), Active, false)]
#[differentiate_ext(dfwd_d, Forward, All(Duplicated), Active, false)]
fn d(x: f32) -> f32 {
    2.0 * x
}
// Generates:
// extern "C" {
//   d_foo(x: f32) -> d_foo_ret;
// }
// #[repr(C)]
// struct d_foo_ret { x:f32, x:f32 }

#[differentiate_ext(d_e, Reverse, PerInput(Duplicated), None, false)]
fn e(x: &mut f32) {
    *x *= 2.0
}

#[differentiate_ext(d_f, Reverse, PerInput(Duplicated, Active), Ignore, false)]
fn f(x: &f32, y: f32) -> f32 {
    *x * y
}

pub fn main() {
    print!("Hello World!");
}
