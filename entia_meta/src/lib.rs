pub mod meta;
pub mod value;

mod examples {
    use super::*;
    use meta::*;
    use value::*;

    #[derive(Clone)]
    pub struct Boba {
        a: usize,
        b: Vec<bool>,
        c: Fett,
    }

    #[derive(Clone)]
    pub struct Jango(usize, Vec<bool>);
    #[derive(Clone)]
    pub struct Jangoz;

    #[derive(Clone)]
    pub enum Fett {
        A(usize),
        B { b: Vec<bool> },
        C,
    }

    impl Meta for Boba {
        #[inline]
        fn meta() -> Type {
            structure!(Boba { a[0]: usize, b[1]: Vec<bool>, c[2]: Fett } [attribute!(derive(Clone))])
        }
    }

    impl Meta for Jango {
        #[inline]
        fn meta() -> Type {
            structure!(Jango(0: usize, 1: Vec<bool>)[attribute!(derive(Clone))])
        }
    }

    impl Meta for Jangoz {
        #[inline]
        fn meta() -> Type {
            structure!(Jangoz[attribute!(derive(Clone))])
        }
    }

    impl Meta for Fett {
        fn meta() -> Type {
            static META: Enumeration = Enumeration {
                module: module_path!(),
                name: "Fett",
                path: std::any::type_name::<Fett>,
                size: std::mem::size_of::<Fett>(),
                file: file!(),
                identifier: std::any::TypeId::of::<Fett>,
                drop: |instance| instance.downcast_mut::<Fett>().map(drop).is_some(),
                index: |name| match name {
                    "A" => Some(0),
                    "B" => Some(1),
                    "C" => Some(2),
                    _ => None,
                },
                index_of: |instance| match instance.downcast_ref()? {
                    Fett::A(..) => Some(0),
                    Fett::B { .. } => Some(1),
                    Fett::C => Some(2),
                },
                attributes: &[attribute!(derive(Clone))],
                variants: &[
                    Variant {
                        name: "A",
                        kind: Structures::Tuple,
                        parent: || &META,
                        values: |instance| match *instance.downcast::<Fett>()? {
                            Fett::A(a) => Ok([Value::from(a)].into()),
                            fett => Err(Box::new(fett)),
                        },
                        attributes: &[],
                        fields: &[Field {
                            name: "0",
                            attributes: &[],
                            meta: usize::meta,
                            parent: Fett::meta,
                            get: |instance| match instance.downcast_ref()? {
                                Fett::A(a) => Some(a),
                                _ => None,
                            },
                            get_mut: |instance| match instance.downcast_mut()? {
                                Fett::A(a) => Some(a),
                                _ => None,
                            },
                            set: |instance, value| match instance.downcast_mut() {
                                Some(Fett::A(a)) => match value.into() {
                                    Some(value) => {
                                        *a = value;
                                        true
                                    }
                                    None => false,
                                },
                                _ => false,
                            },
                        }],
                        index: |name| match name {
                            "0" => Some(0),
                            _ => None,
                        },
                        new: |values| Some(Box::new(Fett::A(values.next()?.into()?))),
                    },
                    Variant {
                        name: "B",
                        kind: Structures::Map,
                        parent: || &META,
                        attributes: &[],
                        fields: &[],
                        values: |instance| match *instance.downcast::<Fett>()? {
                            Fett::B { b } => Ok([Value::from(b)].into()),
                            fett => Err(Box::new(fett)),
                        },
                        new: |values| {
                            Some(Box::new(Fett::B {
                                b: values.next()?.into()?,
                            }))
                        },
                        index: |name| match name {
                            "b" | "0" => Some(0),
                            _ => None,
                        },
                    },
                    Variant {
                        name: "C",
                        kind: Structures::Unit,
                        parent: || &META,
                        attributes: &[],
                        fields: &[],
                        values: |instance| match *instance.downcast::<Fett>()? {
                            Fett::C => Ok([].into()),
                            fett => Err(Box::new(fett)),
                        },
                        new: |_| Some(Box::new(Fett::C)),
                        index: |_| None,
                    },
                ],
            };
            Type::Enumeration(&META)
        }
    }
}
