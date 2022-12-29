use enum_dispatch::enum_dispatch;

macro_rules! enum_dispatch {
    {
        $(#[$meta:meta])*
        $vis:vis enum $name:ident$(<$($lt:lifetime),+>)?: $trait:ident $(+ $add_trait:ident)* {
            $($ty:ident$(<$($item_lt:lifetime)+>)?),*$(,)?
        }
    } => {
        #[enum_dispatch($trait$(, $add_trait)*)]
        $(#[$meta])*
        $vis enum $name$(<$($lt),+>)? {
            $($ty($ty$(<$($item_lt)*>)?),)*
        }
    }
}

#[enum_dispatch]
trait MyTrait {
    fn foo(&self) {}
}

struct Number<'a>(&'a i32);
struct Str<'a>(&'a str);

impl<'a> MyTrait for Number<'a> {}
impl<'a> MyTrait for Str<'a> {}

enum_dispatch! {
    enum MyEnum<'a>: MyTrait {
        Number<'a>,
        Str<'a>,
    }
}
