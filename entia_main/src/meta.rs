use crate::{
    entity::Entity,
    error::{Error, Result},
    resource::Resource,
};
use entia_core::{Maybe, Wrap};
use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    fmt,
    mem::{needs_drop, size_of, ManuallyDrop, MaybeUninit},
    ops::Deref,
    ptr::{copy, drop_in_place, slice_from_raw_parts_mut, NonNull},
    sync::Arc,
};

type Module = dyn Any + Send + Sync;

#[derive(Debug)]
pub struct Metas {
    entity: Arc<Meta>,
    metas: Vec<Arc<Meta>>,
    indices: HashMap<TypeId, usize>,
}

#[derive(Debug)]
pub struct Meta {
    identifier: TypeId,
    name: &'static str,
    pub(crate) allocate: fn(usize) -> NonNull<()>,
    pub(crate) free: unsafe fn(NonNull<()>, usize, usize),
    pub(crate) copy: unsafe fn((NonNull<()>, usize), (NonNull<()>, usize), usize),
    pub(crate) drop: unsafe fn(NonNull<()>, usize, usize),
    pub(crate) defaulter: Option<Defaulter>,
    pub(crate) cloner: Option<Cloner>,
    pub(crate) formatter: Option<Formatter>,
    modules: HashMap<TypeId, Box<Module>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Defaulter {
    pub default: unsafe fn(target: (NonNull<()>, usize), count: usize),
}

#[derive(Debug, Clone)]
pub(crate) struct Cloner {
    pub clone: unsafe fn(source: (NonNull<()>, usize), target: (NonNull<()>, usize), count: usize),
    pub fill: unsafe fn(source: (NonNull<()>, usize), target: (NonNull<()>, usize), count: usize),
}

#[derive(Debug, Clone)]
pub(crate) struct Formatter {
    pub format: unsafe fn(source: NonNull<()>, index: usize) -> String,
}

impl Metas {
    pub fn entity(&self) -> Arc<Meta> {
        self.entity.clone()
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Result<Arc<Meta>> {
        self.get_with(TypeId::of::<T>())
    }

    pub fn get_with(&self, identifier: TypeId) -> Result<Arc<Meta>> {
        match self.indices.get(&identifier) {
            Some(&index) => Ok(self.metas[index].clone()),
            None => Err(Error::MissingMeta { identifier }),
        }
    }

    pub fn get_or_add<T: Send + Sync + 'static>(
        &mut self,
        add: impl FnOnce() -> Meta,
    ) -> Arc<Meta> {
        let identifier = TypeId::of::<T>();
        match self.get_with(identifier) {
            Ok(meta) => meta,
            Err(_) => {
                let meta = Arc::new(add());
                assert!(meta.is::<T>());
                self.indices.insert(identifier, self.metas.len());
                self.metas.push(meta.clone());
                meta
            }
        }
    }
}

impl Default for Metas {
    fn default() -> Self {
        let entity = Arc::new(crate::meta!(Entity));
        let metas = vec![entity.clone()];
        let indices = [(entity.identifier(), 0)].into();
        Self {
            entity,
            metas,
            indices,
        }
    }
}

impl Resource for Metas {}

impl Deref for Metas {
    type Target = [Arc<Meta>];

    fn deref(&self) -> &Self::Target {
        &self.metas
    }
}

impl Meta {
    // To increase safe usage of 'Meta' and 'Store', type 'T' is required to be 'Send + Sync', therefore it is
    // impossible to hold an instance of 'Meta' that does not describe a 'Send + Sync' type.
    pub fn new<T: Send + Sync + 'static, I: IntoIterator<Item = Box<Module>>>(modules: I) -> Self {
        let mut meta = Self {
            identifier: TypeId::of::<T>(),
            name: type_name::<T>(),
            allocate: |capacity| {
                let mut pointer = ManuallyDrop::new(Vec::<T>::with_capacity(capacity));
                unsafe { NonNull::new_unchecked(pointer.as_mut_ptr().cast()) }
            },
            free: |pointer, count, capacity| unsafe {
                Vec::from_raw_parts(pointer.as_ptr().cast::<T>(), count, capacity);
            },
            copy: if size_of::<T>() > 0 {
                |source, target, count| unsafe {
                    if count > 0 {
                        let source = source.0.as_ptr().cast::<T>().add(source.1);
                        let target = target.0.as_ptr().cast::<T>().add(target.1);
                        copy(source, target, count);
                    }
                }
            } else {
                |_, _, _| {}
            },
            drop: if needs_drop::<T>() {
                |pointer, index, count| unsafe {
                    if count > 0 {
                        let pointer = pointer.as_ptr().cast::<T>().add(index);
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
    pub fn is<T: Send + Sync + 'static>(&self) -> bool {
        self.identifier == TypeId::of::<T>()
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

    pub fn default<T: Send + Sync + 'static>(&self) -> Option<T> {
        if self.is::<T>() {
            let defaulter = self.defaulter.as_ref()?;
            Some(unsafe {
                let mut target = MaybeUninit::<T>::uninit();
                (defaulter.default)((NonNull::new_unchecked(target.as_mut_ptr() as _), 0), 1);
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
                let source = NonNull::new_unchecked(value as *const _ as _);
                let mut target = MaybeUninit::<T>::uninit();
                (cloner.clone)(
                    (source, 0),
                    (NonNull::new_unchecked(target.as_mut_ptr() as _), 0),
                    1,
                );
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
                let source = NonNull::new_unchecked(value as *const _ as _);
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
                let target = target.0.as_ptr().cast::<T>().add(target.1);
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
                    let source = source.0.as_ptr().cast::<T>().add(source.1);
                    let target = target.0.as_ptr().cast::<T>().add(target.1);
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
                    let source = &*source.0.as_ptr().cast::<T>().add(source.1);
                    let target = target.0.as_ptr().cast::<T>().add(target.1);
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
            format: |source, index| unsafe {
                format!("{:?}", &*source.as_ptr().cast::<T>().add(index))
            },
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
            std::boxed::Box<dyn std::any::Any + std::marker::Send + std::marker::Sync + 'static>,
        > = std::vec::Vec::new();
        if let Some(module) = $crate::core::Wrap::<$crate::meta::Defaulter, $t>::default().maybe() {
            modules.push(std::boxed::Box::new(module));
        }
        if let Some(module) = $crate::core::Wrap::<$crate::meta::Cloner, $t>::default().maybe() {
            modules.push(std::boxed::Box::new(module));
        }
        if let Some(module) = $crate::core::Wrap::<$crate::meta::Formatter, $t>::default().maybe() {
            modules.push(std::boxed::Box::new(module));
        }
        $crate::meta::Meta::new::<$t, _>(modules)
    }};
}
