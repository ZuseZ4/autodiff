#![allow(unused)]
#![allow(non_camel_case_types)]

use autodiff::differentiate_ext;

// We are currently limited to types compatible to the c-abi,
// therefore we can't pass control as a Vector or Slice directly.

#[differentiate_ext(d_rb_rev, Reverse, PerInput(Gradient, Constant), Ignore, false)]
#[differentiate_ext(d_rb_fwd, Forward, PerInput(Duplicated, Constant), Gradient, false)]
#[differentiate_ext(d_rb_fwd4, Forward(8), PerInput(Duplicated, Constant), Gradient, false)]
fn rosenbrock(control: *mut f64, n: usize) -> f64 {
    let control = unsafe { Vec::from_raw_parts(control, n, n) };
    let b = 100.0;
    let mut result = 0.0;
    for i in (0..n).step_by(2) {
        let c1 = (control[i + 1] - control[i] * control[i]);
        let c2 = 1.0 - control[i];
        result += b * c1 * c1 + c2 * c2;
    }
    result
}

// (Analytical) derivative of the Rosenbrock function
fn rosenbrock_derivative(control: &[f64], derivatives: &mut [f64]) {
    let b = 100.0;
    assert!((control.len() == derivatives.len()));
    for i in (0..control.len()).step_by(2) {
        let c1: f64 = (control[i + 1] - control[i] * control[i]);
        let c2: f64 = 1.0 - control[i];
        derivatives[i + 1] = 2.0 * b * c1;
        derivatives[i] = -4.0 * b * c1 * control[i] - 2.0 * c2;
    }
}
