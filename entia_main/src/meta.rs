use entia_core::{Maybe, Wrap};
use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    fmt,
    mem::{needs_drop, size_of, ManuallyDrop, MaybeUninit},
    ptr::{copy, drop_in_place, slice_from_raw_parts_mut},
};

type Module = dyn Any + Send + Sync;

pub struct Meta {
    identifier: TypeId,
    name: &'static str,
    pub(super) allocate: fn(usize) -> *mut (),
    pub(super) free: unsafe fn(*mut (), usize, usize),
    pub(super) copy: unsafe fn((*const (), usize), (*mut (), usize), usize),
    pub(super) drop: unsafe fn(*mut (), usize, usize),
    pub(super) defaulter: Option<Defaulter>,
    pub(super) cloner: Option<Cloner>,
    pub(super) formatter: Option<Formatter>,
    modules: HashMap<TypeId, Box<Module>>,
}

#[derive(Clone)]
pub struct Defaulter {
    pub default: unsafe fn(target: (*mut (), usize), count: usize),
}

#[derive(Clone)]
pub struct Cloner {
    pub clone: unsafe fn(source: (*const (), usize), target: (*mut (), usize), count: usize),
    pub fill: unsafe fn(source: (*const (), usize), target: (*mut (), usize), count: usize),
}

#[derive(Clone)]
pub struct Formatter {
    pub format: unsafe fn(source: *const (), index: usize) -> String,
}

impl Meta {
    // To increase safe usage of 'Meta' and 'Store', type 'T' is required to be 'Send' and 'Sync', therefore it is
    // impossible to hold an instance of 'Meta' that is not 'Send' and 'Sync'.
    pub fn new<T: Send + Sync + 'static, I: IntoIterator<Item = Box<Module>>>(modules: I) -> Self {
        let mut meta = Self {
            identifier: TypeId::of::<T>(),
            name: type_name::<T>(),
            allocate: |capacity| {
                let mut pointer = ManuallyDrop::new(Vec::<T>::with_capacity(capacity));
                pointer.as_mut_ptr().cast()
            },
            free: |pointer, count, capacity| unsafe {
                Vec::from_raw_parts(pointer.cast::<T>(), count, capacity);
            },
            copy: if size_of::<T>() > 0 {
                |source, target, count| unsafe {
                    if count > 0 {
                        let source = source.0.cast::<T>().add(source.1);
                        let target = target.0.cast::<T>().add(target.1);
                        copy(source, target, count);
                    }
                }
            } else {
                |_, _, _| {}
            },
            drop: if needs_drop::<T>() {
                |pointer, index, count| unsafe {
                    if count > 0 {
                        let pointer = pointer.cast::<T>().add(index);
                        drop_in_place(slice_from_raw_parts_mut(pointer, count));
                    }
                }
            } else {
                |_, _, _| {}
            },
            defaulter: None,
            cloner: None,
            formatter: None,
            modules: modules
                .into_iter()
                .map(|module| (module.type_id(), module))
                .collect(),
        };
        meta.reset();
        meta
    }

    #[inline]
    pub const fn identifier(&self) -> TypeId {
        self.identifier
    }

    #[inline]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.modules
            .get(&TypeId::of::<T>())
            .and_then(|module| module.downcast_ref::<T>())
    }

    pub fn set<T: Send + Sync + 'static>(&mut self, module: T) {
        let module: Box<Module> = Box::new(module);
        self.modules.insert(TypeId::of::<T>(), module);
        self.reset();
    }

    pub fn default<T: 'static>(&self) -> Option<T> {
        if TypeId::of::<T>() == self.identifier() {
            let defaulter = self.defaulter.as_ref()?;
            Some(unsafe {
                let mut target = MaybeUninit::<T>::uninit();
                (defaulter.default)((target.as_mut_ptr() as _, 0), 1);
                target.assume_init()
            })
        } else {
            None
        }
    }

    pub fn clone<T: 'static>(&self, value: &T) -> Option<T> {
        if TypeId::of::<T>() == self.identifier {
            let cloner = self.cloner.as_ref()?;
            Some(unsafe {
                let source = value as *const _ as _;
                let mut target = MaybeUninit::<T>::uninit();
                (cloner.clone)((source, 0), (target.as_mut_ptr() as _, 0), 1);
                target.assume_init()
            })
        } else {
            None
        }
    }

    pub fn format<T: 'static>(&self, value: &T) -> Option<String> {
        if TypeId::of::<T>() == self.identifier {
            let formatter = self.formatter.as_ref()?;
            Some(unsafe {
                let source = value as *const _ as _;
                (formatter.format)(source, 0)
            })
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.defaulter = self.get().cloned();
        self.cloner = self.get().cloned();
        self.formatter = self.get().cloned();
    }
}

impl Defaulter {
    pub fn new<T: Default>() -> Self {
        Self {
            default: |target, count| unsafe {
                let target = target.0.cast::<T>().add(target.1);
                for i in 0..count {
                    target.add(i).write(T::default());
                }
            },
        }
    }
}

impl<T: Default> Maybe<Defaulter> for Wrap<Defaulter, T> {
    fn maybe(self) -> Option<Defaulter> {
        Some(Defaulter::new::<T>())
    }
}

impl Cloner {
    pub fn new<T: Clone>() -> Self {
        Self {
            clone: if size_of::<T>() > 0 {
                |source, target, count| unsafe {
                    let source = source.0.cast::<T>().add(source.1);
                    let target = target.0.cast::<T>().add(target.1);
                    // Use 'ptd::write' to prevent the old value from being dropped since it is expected to be already
                    // dropped or uninitialized.
                    for i in 0..count {
                        let source = &*source.add(i);
                        target.add(i).write(source.clone());
                    }
                }
            } else {
                // TODO: What about implementations of 'Clone' that have side-effects?
                |_, _, _| {}
            },
            fill: if size_of::<T>() > 0 {
                |source, target, count| unsafe {
                    let source = &*source.0.cast::<T>().add(source.1);
                    let target = target.0.cast::<T>().add(target.1);
                    // Use 'ptd::write' to prevent the old value from being dropped since it is expected to be already
                    // dropped or uninitialized.
                    for i in 0..count {
                        target.add(i).write(source.clone());
                    }
                }
            } else {
                // TODO: What about implementations of 'Clone' that have side-effects?
                |_, _, _| {}
            },
        }
    }
}

impl<T: Clone> Maybe<Cloner> for Wrap<Cloner, T> {
    fn maybe(self) -> Option<Cloner> {
        Some(Cloner::new::<T>())
    }
}

impl Formatter {
    pub fn new<T: fmt::Debug>() -> Self {
        Self {
            format: |source, index| unsafe { format!("{:?}", &*source.cast::<T>().add(index)) },
        }
    }
}

impl<T: fmt::Debug> Maybe<Formatter> for Wrap<Formatter, T> {
    fn maybe(self) -> Option<Formatter> {
        Some(Formatter::new::<T>())
    }
}

#[macro_export]
macro_rules! meta {
    ($t:ty) => {{
        use $crate::core::Maybe;

        let mut modules: std::vec::Vec<
            Box<dyn std::any::Any + std::marker::Send + std::marker::Sync + 'static>,
        > = std::vec::Vec::new();

        type Defaulter<T> = $crate::core::Wrap<$crate::meta::Defaulter, T>;
        if let Some(module) = Defaulter::<$t>::default().maybe() {
            modules.push(std::boxed::Box::new(module));
        }
        type Cloner<T> = $crate::core::Wrap<$crate::meta::Cloner, T>;
        if let Some(module) = Cloner::<$t>::default().maybe() {
            modules.push(std::boxed::Box::new(module));
        }
        type Formatter<T> = $crate::core::Wrap<$crate::meta::Formatter, T>;
        if let Some(module) = Formatter::<$t>::default().maybe() {
            modules.push(std::boxed::Box::new(module));
        }

        $crate::meta::Meta::new::<$t, _>(modules)
    }};
}
