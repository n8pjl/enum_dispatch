#[cfg(feature = "extend")]
mod test {
    #[macro_use]
    mod external {
        use enum_dispatch::enum_dispatch;

        pub struct Foo;
        pub struct Bar;

        impl Trait for Foo {}

        impl Trait for Bar {
            fn baz(&self) -> u8 {
                1
            }
        }

        #[enum_dispatch]
        pub trait Trait {
            fn baz(&self) -> u8 {
                0
            }
        }

        #[enum_dispatch(Trait)]
        pub enum TraitDefault {
            Foo,
            Bar,
        }

        pub fn using_trait<T: Trait>(t: &T) -> u8 {
            t.baz()
        }
    }

    use enum_dispatch::{enum_dispatch_extend, enum_dispatch};
    use external::*;

    #[enum_dispatch_extend(Trait, TraitDefault)]
    enum Extended {
        Baz,
    }

    struct Baz {
        num: u8,
    }

    impl Trait for Baz {
        fn baz(&self) -> u8 {
            self.num
        }
    }

    #[test]
    fn main() {
        let foo: Extended = Extended::from(Foo);
        let bar: Extended = Bar.into();
        let baz: Extended = Extended::from(Baz { num: 11 });

        assert_eq!(using_trait(&foo), 0);
        assert_eq!(using_trait(&bar), 1);
        assert_eq!(using_trait(&baz), 11);
    }
}
