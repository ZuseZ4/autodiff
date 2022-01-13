use autodiff::register_derivative;

#[register_derivative(foo, Forward, All(Active), true)]
#[register_derivative(std::module::fnc, Reverse, PerInput(Active, Constant), false)]
#[register_derivative(std::module::fnc, Reverse(Active), All(Gradient), false)]
#[allow(unused)]
fn foo(x: f32) -> f32 {
    2.0 * x
}

pub fn main() {
    print!("Hello World!");
}
