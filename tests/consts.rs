extern crate enum_dispatch;
use enum_dispatch::enum_dispatch;

#[enum_dispatch]
trait HasConst {
    const CONST: char = ' ';
    type Item;

    fn method(&self) -> char {
        Self::CONST
    }
}

struct A;

impl HasConst for A {
    const CONST: char = 'A';
    type Item = char;
}

struct B;

impl HasConst for B {
    const CONST: char = 'B';
    type Item = u32;
}

#[enum_dispatch(HasConst)]
enum Variant {
    A(A),
    B(B),
}

#[test]
fn main() {
    let letter_a = Variant::from(A {});
    let letter_b = Variant::from(B {});
    assert_eq!(letter_a.method(), 'A');
    assert_eq!(letter_b.method(), 'B');
}
