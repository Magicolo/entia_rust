pub mod enumeration;
pub mod field;
pub mod function;
pub mod generic;
pub mod meta;
pub mod module;
pub mod primitive;
pub mod structure;
pub mod r#trait;
pub mod value;
pub mod variant;

pub use self::{
    enumeration::Enumeration,
    field::Field,
    function::{Argument, Function, Parameter},
    generic::Generic,
    meta::{Access, Attribute, Constant, Data, Index, Meta},
    module::Module,
    primitive::Primitive,
    r#trait::Trait,
    structure::Structure,
    value::Value,
    variant::Variant,
};
use entia_meta_macro::meta_extern;
pub use entia_meta_macro::{meta, Meta};

// pub const PHANTOM_DATA: Module = meta_extern!((
//     crate,
//     std,
//     std::marker,
//     r#"C:\Users\strat\.rustup\toolchains\nightly-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\marker.rs"#
// ));

// mod boba {
//     use super::*;
//     use std::{
//         any::{type_name, Any, TypeId},
//         mem::size_of,
//     };

//     pub mod dynamic {
//         use super::*;

//         pub struct Structure {
//             pub access: Access,
//             pub name: &'static str,
//             pub path: fn() -> &'static str,
//             pub size: usize,
//             pub identifier: fn() -> TypeId,
//             pub new: Option<fn(&mut dyn Iterator<Item = Value>) -> Option<Box<dyn Any>>>,
//             pub values: Option<fn(Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>>>,
//             pub attributes: &'static [Attribute],
//             pub generics: &'static [Generic],
//             pub fields: &'static [Field],
//         }

//         pub struct Field {}
//         pub struct Generic {}
//         pub struct Attribute {}
//     }

//     pub mod r#static {
//         use std::{
//             marker::PhantomData,
//             ops::{Deref, DerefMut},
//         };

//         use self::{
//             attribute::Attribute,
//             field::Field,
//             generic::Generic,
//             member::Member,
//             one::{One, With},
//             unit::Unit,
//             variant::Variant,
//         };
//         use super::*;

//         pub enum Kind {
//             Unit,
//             Primitive,
//             Structure,
//             Enumeration,
//         }

//         pub trait Meta {
//             type Instance;
//             type Type: Type<Instance = Self::Instance>;
//             fn meta() -> Self::Type;
//         }

//         pub trait Value {
//             type Instance;
//             type Type: Type<Instance = Self::Instance>;
//             fn get_meta(&self) -> Self::Type;
//         }

//         impl<T: Meta> Value for T {
//             type Instance = T::Instance;
//             type Type = T::Type;

//             fn get_meta(&self) -> Self::Type {
//                 T::meta()
//             }
//         }

//         pub trait Type {
//             type Instance;

//             fn kind(&self) -> Kind;
//             fn access(&self) -> Access;
//             fn name(&self) -> &'static str;
//             fn identifier(&self) -> Option<TypeId>;

//             fn path(&self) -> &'static str {
//                 type_name::<Self::Instance>()
//             }

//             fn size(&self) -> usize {
//                 size_of::<Self::Instance>()
//             }

//             fn attribute<A: Attribute<Self::Instance>, K: Key<A>>(&self, key: K) -> A {
//                 key.value()
//             }

//             fn field<F: Field<Self::Instance>, K: Key<F>>(&self, key: K) -> F {
//                 key.value()
//             }

//             fn variant<V: Variant<Self::Instance>, K: Key<V>>(&self, key: K) -> V {
//                 key.value()
//             }

//             fn generic<G: Generic<Self::Instance>, K: Key<G>>(&self, key: K) -> G {
//                 key.value()
//             }

//             fn member<M: Member<Self::Instance>, K: Key<M>>(&self, key: K) -> M {
//                 key.value()
//             }
//         }

//         pub trait Key<V> {
//             fn value(self) -> V;
//         }

//         pub trait Get<T> {
//             type Value: Value;
//             fn get(&self, instance: T) -> Self::Value;
//         }

//         macro_rules! get {
//             ($t:ty, $f:ty, $v:ty, $n:tt) => {
//                 impl Get<$t> for $f {
//                     type Value = $v;

//                     fn get(&self, instance: $t) -> Self::Value {
//                         instance.$n
//                     }
//                 }

//                 impl<'a> Get<&'a $t> for $f {
//                     type Value = &'a $v;

//                     fn get(&self, instance: &'a $t) -> Self::Value {
//                         &instance.$n
//                     }
//                 }

//                 impl<'a> Get<&'a mut $t> for $f {
//                     type Value = &'a mut $v;

//                     fn get(&self, instance: &'a mut $t) -> Self::Value {
//                         &mut instance.$n
//                     }
//                 }
//             };
//         }

//         pub mod member {
//             use super::*;

//             pub trait Member<T> {
//                 type Value: Value;
//                 fn name(&self) -> &'static str;
//                 fn get(&self, instance: T) -> Self::Value;
//             }
//         }

//         pub mod field {
//             use super::*;

//             pub trait Field<T>: Get<T> + for<'a> Get<&'a T> + for<'a> Get<&'a mut T> {
//                 fn name(&self) -> &'static str;
//             }

//             // pub trait Field {
//             //     type Instance;
//             //     type Get: Value;
//             //     type GetRef<'a>: Value;
//             //     type GetMut<'a>: Value;
//             //     fn name(&self) -> &'static str;
//             //     fn get(&self, instance: Self::Instance) -> Self::Get;
//             //     fn get_ref<'a>(&self, instance: &'a Self::Instance) -> Self::GetRef<'a>;
//             //     fn get_mut<'a>(&self, instance: &'a mut Self::Instance) -> Self::GetMut<'a>;
//             // }
//         }

//         pub mod variant {
//             use super::*;

//             pub trait Variant<T> {
//                 fn name(&self) -> &'static str;
//                 fn is(&self, instance: T) -> bool;
//             }
//         }

//         pub mod generic {
//             use super::*;

//             pub trait Generic<T> {
//                 fn name(&self) -> &'static str;
//             }
//         }

//         pub mod attribute {
//             use super::*;

//             pub trait Attribute<T> {
//                 fn name(&self) -> &'static str;
//                 fn content(&self) -> &'static str;
//             }
//         }

//         pub struct UsizeType;
//         pub struct BoolType;

//         impl<T: Meta> Meta for &T {
//             type Instance = T::Instance;
//             type Type = T::Type;

//             fn meta() -> Self::Type {
//                 T::meta()
//             }
//         }

//         impl<T: Meta> Meta for &mut T {
//             type Instance = T::Instance;
//             type Type = T::Type;

//             fn meta() -> Self::Type {
//                 T::meta()
//             }
//         }

//         pub mod primitive {
//             use super::*;

//             impl Meta for usize {
//                 type Instance = Self;
//                 type Type = UsizeType;

//                 fn meta() -> Self::Type {
//                     UsizeType
//                 }
//             }

//             impl Meta for bool {
//                 type Instance = Self;
//                 type Type = BoolType;

//                 fn meta() -> Self::Type {
//                     BoolType
//                 }
//             }

//             impl Type for UsizeType {
//                 type Instance = usize;

//                 fn kind(&self) -> Kind {
//                     Kind::Primitive
//                 }

//                 fn access(&self) -> Access {
//                     Access::Public
//                 }

//                 fn name(&self) -> &'static str {
//                     "usize"
//                 }

//                 fn identifier(&self) -> Option<TypeId> {
//                     Some(TypeId::of::<usize>())
//                 }
//             }

//             impl Type for BoolType {
//                 type Instance = bool;

//                 fn kind(&self) -> Kind {
//                     Kind::Primitive
//                 }

//                 fn access(&self) -> Access {
//                     Access::Public
//                 }

//                 fn name(&self) -> &'static str {
//                     "bool"
//                 }

//                 fn identifier(&self) -> Option<TypeId> {
//                     Some(TypeId::of::<bool>())
//                 }
//             }
//         }

//         pub mod one {
//             use super::*;

//             pub enum One<T1, T2> {
//                 A(T1),
//                 B(T2),
//             }
//             pub struct With<I, O>(pub I, PhantomData<O>);

//             impl<I, O> Deref for With<I, O> {
//                 type Target = I;

//                 fn deref(&self) -> &Self::Target {
//                     &self.0
//                 }
//             }

//             impl<I, O> DerefMut for With<I, O> {
//                 fn deref_mut(&mut self) -> &mut Self::Target {
//                     &mut self.0
//                 }
//             }

//             impl<I, O> From<I> for With<I, O> {
//                 fn from(inner: I) -> Self {
//                     Self(inner, PhantomData)
//                 }
//             }

//             impl<V1: Value, V2: Value> Value for One<V1, V2> {
//                 type Instance = One<V1::Instance, V2::Instance>;
//                 type Type = One<V1::Type, V2::Type>;

//                 fn get_meta(&self) -> Self::Type {
//                     match self {
//                         One::A(a) => One::A(a.get_meta()),
//                         One::B(b) => One::B(b.get_meta()),
//                     }
//                 }
//             }

//             impl<T, V1: Value<Instance = T>, V2: Value<Instance = T>> Value for With<One<V1, V2>, T> {
//                 type Instance = T;
//                 type Type = With<One<V1::Type, V2::Type>, T>;

//                 fn get_meta(&self) -> Self::Type {
//                     match &**self {
//                         One::A(a) => One::A(a.get_meta()),
//                         One::B(b) => One::B(b.get_meta()),
//                     }
//                     .into()
//                 }
//             }

//             impl<T1: Type, T2: Type> Type for One<T1, T2> {
//                 type Instance = One<T1::Instance, T2::Instance>;

//                 fn kind(&self) -> Kind {
//                     match self {
//                         One::A(a) => a.kind(),
//                         One::B(b) => b.kind(),
//                     }
//                 }

//                 fn access(&self) -> Access {
//                     match self {
//                         One::A(a) => a.access(),
//                         One::B(b) => b.access(),
//                     }
//                 }

//                 fn name(&self) -> &'static str {
//                     match self {
//                         One::A(a) => a.name(),
//                         One::B(b) => b.name(),
//                     }
//                 }

//                 fn identifier(&self) -> Option<TypeId> {
//                     match self {
//                         One::A(a) => a.identifier(),
//                         One::B(b) => b.identifier(),
//                     }
//                 }

//                 fn path(&self) -> &'static str {
//                     match self {
//                         One::A(a) => a.path(),
//                         One::B(b) => b.path(),
//                     }
//                 }

//                 fn size(&self) -> usize {
//                     match self {
//                         One::A(a) => a.size(),
//                         One::B(b) => b.size(),
//                     }
//                 }
//             }

//             impl<T, T1: Type<Instance = T>, T2: Type<Instance = T>> Type for With<One<T1, T2>, T> {
//                 type Instance = T;

//                 fn kind(&self) -> Kind {
//                     match &**self {
//                         One::A(a) => a.kind(),
//                         One::B(b) => b.kind(),
//                     }
//                 }

//                 fn access(&self) -> Access {
//                     match &**self {
//                         One::A(a) => a.access(),
//                         One::B(b) => b.access(),
//                     }
//                 }

//                 fn name(&self) -> &'static str {
//                     match &**self {
//                         One::A(a) => a.name(),
//                         One::B(b) => b.name(),
//                     }
//                 }

//                 fn identifier(&self) -> Option<TypeId> {
//                     match &**self {
//                         One::A(a) => a.identifier(),
//                         One::B(b) => b.identifier(),
//                     }
//                 }

//                 fn path(&self) -> &'static str {
//                     match &**self {
//                         One::A(a) => a.path(),
//                         One::B(b) => b.path(),
//                     }
//                 }

//                 fn size(&self) -> usize {
//                     match &**self {
//                         One::A(a) => a.size(),
//                         One::B(b) => b.size(),
//                     }
//                 }
//             }

//             impl<T1, T2, M1: Member<T1>, M2: Member<T2>> Member<One<T1, T2>> for One<M1, M2> {
//                 type Value = One<One<M1::Value, M2::Value>, Unit>;

//                 fn name(&self) -> &'static str {
//                     match self {
//                         One::A(a) => a.name(),
//                         One::B(b) => b.name(),
//                     }
//                 }

//                 fn get(&self, instance: One<T1, T2>) -> Self::Value {
//                     match (self, instance) {
//                         (One::A(member), One::A(instance)) => One::A(One::A(member.get(instance))),
//                         (One::B(member), One::B(instance)) => One::A(One::B(member.get(instance))),
//                         _ => One::B(Unit),
//                     }
//                 }
//             }

//             impl<V1, V2, K1: Key<V1>, K2: Key<V2>> Key<One<V1, V2>> for One<K1, K2> {
//                 fn value(self) -> One<V1, V2> {
//                     match self {
//                         One::A(a) => One::A(a.value()),
//                         One::B(b) => One::B(b.value()),
//                     }
//                 }
//             }

//             impl<V, K1: Key<V>, K2: Key<V>> Key<V> for With<One<K1, K2>, V> {
//                 fn value(self) -> V {
//                     match self.0 {
//                         One::A(a) => a.value(),
//                         One::B(b) => b.value(),
//                     }
//                 }
//             }

//             impl<T1, T2, F1: Field<T1>, F2: Field<T2>> Field<One<T1, T2>> for One<F1, F2> {
//                 fn name(&self) -> &'static str {
//                     match self {
//                         One::A(a) => a.name(),
//                         One::B(b) => b.name(),
//                     }
//                 }
//             }

//             impl<T, F1: Field<T>, F2: Field<T>> Field<T> for With<One<F1, F2>, T> {
//                 fn name(&self) -> &'static str {
//                     (&**self).name()
//                 }
//             }

//             impl<T1, T2, G1: Get<T1>, G2: Get<T2>> Get<One<T1, T2>> for One<G1, G2> {
//                 type Value = One<One<G1::Value, G2::Value>, Unit>;

//                 fn get(&self, instance: One<T1, T2>) -> Self::Value {
//                     match (self, instance) {
//                         (One::A(a), One::A(instance)) => One::A(One::A(a.get(instance))),
//                         (One::B(b), One::B(instance)) => One::A(One::B(b.get(instance))),
//                         _ => One::B(Unit),
//                     }
//                 }
//             }

//             // impl<'a, T1, T2, G1: Get<&'a T1>, G2: Get<&'a T2>> Get<One<&'a T1, &'a T2>> for One<G1, G2> {
//             //     type Value = One<One<G1::Value, G2::Value>, Unit>;

//             //     fn get(&self, instance: One<&'a T1, &'a T2>) -> Self::Value {
//             //         let a = Some(2u16).as_ref();
//             //         match (self, instance) {
//             //             (One::A(a), One::A(instance)) => One::A(One::A(a.get(instance))),
//             //             (One::B(b), One::B(instance)) => One::A(One::B(b.get(instance))),
//             //             _ => One::B(Unit),
//             //         }
//             //     }
//             // }

//             impl<T, G1: Get<T>, G2: Get<T>> Get<T> for With<One<G1, G2>, T> {
//                 type Value = One<G1::Value, G2::Value>;

//                 fn get(&self, instance: T) -> Self::Value {
//                     match &**self {
//                         One::A(a) => One::A(a.get(instance)),
//                         One::B(b) => One::B(b.get(instance)),
//                     }
//                 }
//             }

//             impl<'a, T, G1: Get<&'a T>, G2: Get<&'a T>> Get<&'a T> for With<One<G1, G2>, T> {
//                 type Value = One<G1::Value, G2::Value>;

//                 fn get(&self, instance: &'a T) -> Self::Value {
//                     match &**self {
//                         One::A(a) => One::A(a.get(instance)),
//                         One::B(b) => One::B(b.get(instance)),
//                     }
//                 }
//             }

//             impl<'a, T, G1: Get<&'a mut T>, G2: Get<&'a mut T>> Get<&'a mut T> for With<One<G1, G2>, T> {
//                 type Value = One<G1::Value, G2::Value>;

//                 fn get(&self, instance: &'a mut T) -> Self::Value {
//                     match &**self {
//                         One::A(a) => One::A(a.get(instance)),
//                         One::B(b) => One::B(b.get(instance)),
//                     }
//                 }
//             }
//         }

//         pub mod unit {
//             use super::*;

//             pub struct Unit;

//             impl Key<Unit> for usize {
//                 fn value(self) -> Unit {
//                     Unit
//                 }
//             }

//             impl<T> Member<T> for Unit {
//                 type Value = Unit;

//                 fn name(&self) -> &'static str {
//                     ""
//                 }

//                 fn get(&self, _: T) -> Self::Value {
//                     Unit
//                 }
//             }

//             impl<T> Get<T> for Unit {
//                 type Value = Self;

//                 fn get(&self, _: T) -> Self::Value {
//                     Self
//                 }
//             }

//             impl<T> Field<T> for Unit {
//                 fn name(&self) -> &'static str {
//                     ""
//                 }
//             }

//             impl Meta for Unit {
//                 type Instance = Unit;
//                 type Type = Unit;

//                 fn meta() -> Self::Type {
//                     Unit
//                 }
//             }

//             impl Type for Unit {
//                 type Instance = Unit;

//                 fn kind(&self) -> Kind {
//                     Kind::Unit
//                 }

//                 fn access(&self) -> Access {
//                     Access::Public
//                 }

//                 fn name(&self) -> &'static str {
//                     ""
//                 }

//                 fn identifier(&self) -> Option<TypeId> {
//                     None
//                 }

//                 fn path(&self) -> &'static str {
//                     ""
//                 }

//                 fn size(&self) -> usize {
//                     0
//                 }
//             }
//         }

//         pub mod boba {
//             use super::*;

//             pub struct Boba {
//                 pub a: usize,
//                 pub b: bool,
//             }

//             pub struct BobaType;
//             pub struct AKey;
//             pub struct AField;
//             pub struct BKey;
//             pub struct BField;
//             pub enum AnyKey {
//                 A,
//                 B,
//             }

//             impl Type for BobaType {
//                 type Instance = Boba;

//                 fn kind(&self) -> Kind {
//                     Kind::Structure
//                 }

//                 fn access(&self) -> Access {
//                     Access::Public
//                 }

//                 fn name(&self) -> &'static str {
//                     "Boba"
//                 }

//                 fn identifier(&self) -> Option<TypeId> {
//                     Some(TypeId::of::<Boba>())
//                 }
//             }

//             impl<T> Key<With<One<AField, BField>, T>> for AnyKey {
//                 fn value(self) -> With<One<AField, BField>, T> {
//                     match self {
//                         AnyKey::A => One::A(AField),
//                         AnyKey::B => One::B(BField),
//                     }
//                     .into()
//                 }
//             }

//             impl Key<AField> for AKey {
//                 fn value(self) -> AField {
//                     AField
//                 }
//             }

//             impl Field<Boba> for AField {
//                 // type Value = usize;

//                 fn name(&self) -> &'static str {
//                     "a"
//                 }

//                 // fn get(&self, instance: Boba) -> Self::Value {
//                 //     instance.a
//                 // }
//             }

//             impl Field<Boba> for BField {
//                 fn name(&self) -> &'static str {
//                     "b"
//                 }
//             }

//             get!(Boba, AField, usize, a);
//             get!(Boba, BField, bool, b);

//             impl Key<BField> for BKey {
//                 fn value(self) -> BField {
//                     BField
//                 }
//             }

//             impl<T> Key<One<With<One<AField, BField>, T>, Unit>> for usize {
//                 fn value(self) -> One<With<One<AField, BField>, T>, Unit> {
//                     match self {
//                         0 => One::A(One::A(AField).into()),
//                         1 => One::A(One::B(BField).into()),
//                         _ => One::B(Unit),
//                     }
//                 }
//             }

//             impl<T> Key<With<One<With<One<AField, BField>, T>, Unit>, T>> for &str {
//                 fn value(self) -> With<One<With<One<AField, BField>, T>, Unit>, T> {
//                     match self {
//                         "a" | "0" => One::A(One::A(AField).into()),
//                         "b" | "1" => One::A(One::B(BField).into()),
//                         _ => One::B(Unit),
//                     }
//                     .into()
//                 }
//             }

//             impl Meta for Boba {
//                 type Instance = Self;
//                 type Type = BobaType;

//                 fn meta() -> Self::Type {
//                     BobaType
//                 }
//             }

//             fn _test() {
//                 let boba = Boba { a: 0, b: true };
//                 let _a = Boba::meta().field(AKey).get(&boba);
//                 let _a = Boba::meta().field(AnyKey::A).get(&boba);
//                 let _a = Boba::meta().field("A");
//                 // let _a = Boba::meta()
//                 //     .member(AKey)
//                 //     .get(Boba { a: 0, b: false })
//                 //     .get_meta();
//                 // let _a = Boba::meta().member(BKey);
//                 // let _a = Boba::meta().member(Boba::a);
//                 // let _a = boba
//                 //     .get_meta()
//                 //     .member(Boba::a)
//                 //     .get(&boba)
//                 //     .get_meta()
//                 //     .member(One::<usize, usize>::A(0))
//                 //     .get_meta();
//                 // let _a = Boba::meta().member(Boba::b);
//                 // let _a = Boba::meta().member(0).get(&boba);
//                 // let _a = Boba::meta().member(0).get(&boba).get_meta();
//                 // let _a = Boba::meta().member("a");
//                 // let _a = _karl(boba);
//             }
//         }

//         pub mod fett {
//             use std::marker::PhantomData;

//             use super::*;

//             pub enum Fett<T> {
//                 A(T),
//                 B { a: usize, b: bool, boba: boba::Boba },
//             }

//             pub mod keys {
//                 use super::*;

//                 pub enum Variant {
//                     A,
//                     B,
//                 }

//                 // pub enum Parameter {
//                 //     T,
//                 // }

//                 // impl<T> Key<Fett<T>> for Parameter {
//                 //     type Value = parameters::T;

//                 //     fn get(self) -> Self::Value {
//                 //         todo!()
//                 //     }
//                 // }

//                 pub struct A;
//                 pub struct B;

//                 impl<T: Value> Key<variants::A<T>> for A {
//                     fn value(self) -> variants::A<T> {
//                         variants::A(PhantomData)
//                     }
//                 }

//                 impl Key<variants::B> for B {
//                     fn value(self) -> variants::B {
//                         variants::B
//                     }
//                 }

//                 impl<T: Value> Key<With<One<variants::A<T>, variants::B>, Fett<T>>> for Variant {
//                     fn value(self) -> With<One<variants::A<T>, variants::B>, Fett<T>> {
//                         match self {
//                             Variant::A => One::A(variants::A(PhantomData)),
//                             Variant::B => One::B(variants::B),
//                         }
//                         .into()
//                     }
//                 }

//                 impl<T: Value> Key<One<With<One<variants::A<T>, variants::B>, Fett<T>>, Unit>> for usize {
//                     fn value(self) -> One<With<One<variants::A<T>, variants::B>, Fett<T>>, Unit> {
//                         match self {
//                             0 => One::A(One::A(variants::A(PhantomData)).into()),
//                             1 => One::A(One::B(variants::B).into()),
//                             _ => One::B(Unit),
//                         }
//                     }
//                 }

//                 impl<T: Value>
//                     Key<With<One<With<One<variants::A<T>, variants::B>, Fett<T>>, Unit>, Fett<T>>>
//                     for &str
//                 {
//                     fn value(
//                         self,
//                     ) -> With<One<With<One<variants::A<T>, variants::B>, Fett<T>>, Unit>, Fett<T>>
//                     {
//                         match self {
//                             "A" | "0" => One::A(One::A(variants::A(PhantomData)).into()),
//                             "B" | "1" => One::A(One::B(variants::B).into()),
//                             _ => One::B(Unit),
//                         }
//                         .into()
//                     }
//                 }
//             }

//             pub mod generics {
//                 use super::*;

//                 pub struct T;
//             }

//             pub mod values {
//                 use super::*;

//                 pub struct A<T>(pub T);
//                 pub struct B {
//                     pub a: usize,
//                     pub b: bool,
//                     pub boba: boba::Boba,
//                 }

//                 pub struct AType<T>(PhantomData<T>);
//                 impl<T> Meta for A<T> {
//                     type Instance = Self;
//                     type Type = AType<T>;

//                     fn meta() -> Self::Type {
//                         AType(PhantomData)
//                     }
//                 }

//                 impl<T> Type for AType<T> {
//                     type Instance = A<T>;

//                     fn kind(&self) -> Kind {
//                         todo!()
//                     }

//                     fn access(&self) -> Access {
//                         Access::Public
//                     }

//                     fn identifier(&self) -> Option<TypeId> {
//                         None
//                     }

//                     fn name(&self) -> &'static str {
//                         "A"
//                     }
//                 }

//                 pub struct BType;
//                 impl Meta for B {
//                     type Instance = Self;
//                     type Type = BType;

//                     fn meta() -> Self::Type {
//                         BType
//                     }
//                 }

//                 impl Type for BType {
//                     type Instance = B;

//                     fn kind(&self) -> Kind {
//                         todo!()
//                     }

//                     fn access(&self) -> Access {
//                         Access::Public
//                     }

//                     fn name(&self) -> &'static str {
//                         "B"
//                     }

//                     fn identifier(&self) -> Option<TypeId> {
//                         Some(TypeId::of::<Self::Instance>())
//                     }
//                 }
//             }

//             pub mod variants {
//                 use super::*;

//                 pub struct A<T>(pub PhantomData<T>);
//                 pub struct B;

//                 impl<T: Value> Member<Fett<T>> for A<T> {
//                     type Value = One<values::A<T>, Unit>;

//                     fn name(&self) -> &'static str {
//                         "A"
//                     }

//                     fn get(&self, instance: Fett<T>) -> Self::Value {
//                         match instance {
//                             Fett::A(a) => One::A(values::A(a)),
//                             Fett::B { .. } => One::B(Unit),
//                         }
//                     }
//                 }

//                 impl<T> Member<Fett<T>> for B {
//                     type Value = One<Unit, values::B>;

//                     fn name(&self) -> &'static str {
//                         "B"
//                     }

//                     fn get(&self, instance: Fett<T>) -> Self::Value {
//                         match instance {
//                             Fett::A(_) => One::A(Unit),
//                             Fett::B { a, b, boba } => One::B(values::B { a, b, boba }),
//                         }
//                     }
//                 }
//             }

//             pub struct FettType<T>(PhantomData<T>);

//             impl<T> Meta for Fett<T> {
//                 type Instance = Self;
//                 type Type = FettType<T>;

//                 fn meta() -> Self::Type {
//                     FettType(PhantomData)
//                 }
//             }

//             impl<T> Type for FettType<T> {
//                 type Instance = Fett<T>;

//                 fn kind(&self) -> Kind {
//                     Kind::Enumeration
//                 }

//                 fn access(&self) -> Access {
//                     Access::Public
//                 }

//                 fn name(&self) -> &'static str {
//                     "Fett"
//                 }

//                 fn identifier(&self) -> Option<TypeId> {
//                     None
//                 }
//             }

//             fn _test() {
//                 let fett = Fett::A(0);
//                 let _a = Fett::<usize>::meta();
//                 let _a = Fett::<usize>::meta().member(keys::A);
//                 let _a = Fett::<usize>::meta().member(keys::A).get(Fett::A(0));
//                 let _a = Fett::<usize>::meta()
//                     .member(keys::A)
//                     .get(Fett::A(0))
//                     .get_meta();
//                 let _a = Fett::<usize>::meta().member(keys::B);
//                 let _a = Fett::<usize>::meta().member(keys::B).get(Fett::A(0));
//                 let _a = Fett::<usize>::meta()
//                     .member(keys::B)
//                     .get(Fett::A(0))
//                     .get_meta();
//                 // _karl(fett);
//             }
//         }

//         fn _karl<T: Meta, F: Field<T::Instance>>(_: T) -> (F, F)
//         where
//             usize: Key<F>,
//             &'static str: Key<F>,
//         {
//             (T::meta().field(0), T::meta().field("A"))
//         }
//     }
// }
