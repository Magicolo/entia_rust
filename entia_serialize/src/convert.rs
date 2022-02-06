use crate::{
    node::Node::{self, *},
    recurse,
};

pub trait Convert {
    type Item: 'static;
    fn to(&self) -> Node;
    fn from(node: &Node) -> Self::Item;
}

impl<C: Convert> Convert for &C {
    type Item = C::Item;

    fn to(&self) -> Node {
        C::to(self)
    }

    fn from(node: &Node) -> Self::Item {
        C::from(node)
    }
}

impl<C: Convert> Convert for &mut C {
    type Item = C::Item;

    fn to(&self) -> Node {
        C::to(self)
    }

    fn from(node: &Node) -> Self::Item {
        C::from(node)
    }
}

impl Convert for std::string::String {
    type Item = Self;

    fn to(&self) -> Node {
        String(self.clone())
    }

    fn from(node: &Node) -> Self::Item {
        node.string().unwrap_or_default().into()
    }
}

pub struct Iterator<I>(I);

impl<I: IntoIterator> Iterator<I> {
    pub fn new(iterator: I) -> Self {
        Self(iterator)
    }
}

impl<I: 'static> Convert for Iterator<I>
where
    for<'a> &'a I: IntoIterator,
    for<'a> <&'a I as IntoIterator>::Item: Convert,
    I: for<'a> FromIterator<<<&'a I as IntoIterator>::Item as Convert>::Item>,
{
    type Item = I;

    fn to(&self) -> Node {
        Array(self.0.into_iter().map(|value| value.to()).collect())
    }

    fn from(node: &Node) -> Self::Item {
        match node {
            Array(nodes) => nodes
                .iter()
                .map(|node| <<&I as IntoIterator>::Item as Convert>::from(node))
                .collect(),
            _ => [].into_iter().collect(),
        }
    }
}

/*
struct Karl {
    a: bool,
    b: usize,
    c: Vec<bool>,
}

impl Convert for Karl {
    type Item = Self;

    fn to(&self) -> Node {
        Object(vec![(String("a".into()), <bool as Convert>::to(&self.a))])
    }

    fn from(node: &Node) -> Self::Item {
        object(
            node,
            |node| {
                let values = <(bool, usize, Vec<bool>) as Convert>::from(node);
                Karl {
                    a: values.0,
                    b: values.1,
                    c: values.2,
                }
            },
            |instance, key, value| match key.string() {
                Some("a") => {
                    instance.a = <bool as Convert>::from(value);
                    true
                }
                Some("b") => {
                    instance.b = <usize as Convert>::from(value);
                    true
                }
                Some("c") => {
                    instance.c = <Vec<bool> as Convert>::from(value);
                    true
                }
                _ => match key.integer() {
                    Some(0) => {
                        instance.a = <bool as Convert>::from(value);
                        true
                    }
                    Some(1) => {
                        instance.b = <usize as Convert>::from(value);
                        true
                    }
                    Some(2) => {
                        instance.c = <Vec<bool> as Convert>::from(value);
                        true
                    }
                    _ => false,
                },
            },
        )
    }
}
*/

pub fn object<T>(
    node: &Node,
    default: impl FnOnce(&Node) -> T,
    apply: impl Fn(&mut T, &Node, &Node) -> bool,
) -> T {
    let mut instance = default(node);
    match node {
        // TODO: Maybe the default instantiation can be skipped here?
        Object(nodes) => {
            for (key, value) in nodes {
                match key {
                    // Support for multi-keys.
                    Array(keys) => {
                        for key in keys {
                            if apply(&mut instance, key, value) {
                                break;
                            }
                        }
                    }
                    _ => {
                        apply(&mut instance, key, value);
                    }
                }
            }
            instance
        }
        _ => instance,
    }
}

macro_rules! primitive {
    ($t:ident, $n:ident, $c:ident) => {
        impl Convert for $t {
            type Item = Self;

            #[inline]
            fn to(&self) -> Node {
                $n(*self as _)
            }

            #[inline]
            fn from(node: &Node) -> Self::Item {
                node.$c().unwrap_or_default() as _
            }
        }
    };
}

macro_rules! integer {
    ($($t:ident),*) => {
        $(primitive!($t, Integer, integer);)*
    };
}

macro_rules! floating {
    ($($t:ident),*) => {
        $(primitive!($t, Floating, floating);)*
    };
}

macro_rules! tuple {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Convert,)*> Convert for ($($t,)*) {
            type Item = ($($t::Item,)*);

            #[inline]
            fn to(&self) -> Node {
                let ($($p,)*) = self;
                Array(vec![$(<$t as Convert>::to($p),)*])
            }

            #[inline]
            fn from(node: &Node) -> Self::Item {
                match node {
                    Array(_nodes) => {
                        let mut _iterator = _nodes.into_iter();
                        $(let $p = <$t as Convert>::from(_iterator.next().unwrap_or(&Null));)*
                        ($($p,)*)
                    }
                    _ => {
                        $(let $p = <$t as Convert>::from(&Null);)*
                        ($($p,)*)
                    },
                }
            }
        }
    };
}

primitive!(bool, Boolean, boolean);
integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
floating!(f32, f64);
recurse!(tuple);

#[test]
fn boba() {
    let node = Iterator::new(vec![true, false]).to();
    assert!(match node {
        Array(items) if items.len() == 2 => match (&items[0], &items[1]) {
            (Boolean(true), Boolean(false)) => true,
            _ => false,
        },
        _ => false,
    });
}
