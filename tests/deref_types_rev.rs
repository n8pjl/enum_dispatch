use enum_dispatch::enum_dispatch;
use std::rc::Rc;
use std::sync::Arc;
 
#[enum_dispatch(MyTrait)]
enum Enum {
    A,
    #[enum_dispatch(deref)]
    B1(Rc<B>),
    #[enum_dispatch(deref)]
    B2(Arc<B>),
    // make sure we can still leverage cfg attributes
    #[enum_dispatch(deref)]
    #[cfg(test)]
    C1(Rc<A>),
    #[cfg(not(test))]
    C2(Rc<A>),
}

#[enum_dispatch]
pub trait MyTrait {
    fn do_something(&self) -> &'static str;
}

pub struct A;

impl MyTrait for A {
    fn do_something(&self) -> &'static str {
        "a"
    }
}

impl MyTrait for B {
    fn do_something(&self) -> &'static str {
        "b"
    }
}

pub struct B;


#[test]
fn main() {
    let a = Enum::A(A {});
    let b1 = Enum::B1(Rc::new(B {}));
    let b2 = Enum::B2(Arc::new(B {}));
    let c1 = Enum::C1(Rc::new(A {}));

    assert_eq!(a.do_something(), "a");
    assert_eq!(b1.do_something(), "b");
    assert_eq!(b2.do_something(), "b");
    assert_eq!(c1.do_something(), "a");
}
