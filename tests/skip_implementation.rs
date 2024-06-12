use enum_dispatch::enum_dispatch;

#[enum_dispatch(Traited)]
trait Trait {
    fn describe(&self) -> String;
}

impl Trait for TestType {
    fn describe(&self) -> String {
        "TestType".to_string()
    }
}

pub struct TestType;

#[enum_dispatch(skip_from, skip_try_into)]
pub enum Traited {
    A(TestType),
    B(TestType),
    C(TestType),
}

#[test]
fn main() {
    let a = Traited::A(TestType {});
    let b = Traited::B(TestType {});
    let c = Traited::C(TestType {});

    assert_eq!(a.describe(), "TestType");
    assert_eq!(b.describe(), "TestType");
    assert_eq!(c.describe(), "TestType");
}
