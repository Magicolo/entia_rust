// use crate::{
//     deserializer::{Deserializer, Primitive},
//     serializer::{self, Serializer},
// };

// pub struct Cloner<S, D>(S, D);

// impl<S: Serializer, D: Deserializer<Error = S::Error>> Deserializer for Cloner<S, D> {
//     type Error = S::Error;
//     type Primitive = Cloner<S::Primitive, D::Primitive>;
//     type Structure = Cloner<S::Structure, D::Structure>;
//     type Sequence = Cloner<S::Sequence, D::Sequence>;
//     type Enumeration = Cloner<S::Enumeration, D::Enumeration>;

//     fn primitive(self) -> Result<Self::Primitive, Self::Error> {
//         Ok(Cloner(self.0.primitive()?, self.1.primitive()?))
//     }

//     fn structure(self, name: &'static str) -> Result<Self::Structure, Self::Error> {
//         todo!()
//     }

//     fn enumeration(self, name: &'static str) -> Result<Self::Enumeration, Self::Error> {
//         todo!()
//     }

//     fn sequence(self) -> Result<Self::Sequence, Self::Error> {
//         todo!()
//     }
// }

// impl<S: serializer::Primitive, D: Primitive<Error = S::Error>> Primitive for Cloner<S, D> {
//     type Error = S::Error;

//     fn unit(self) -> Result<(), Self::Error> {
//         todo!()
//     }

//     fn bool(self) -> Result<bool, Self::Error> {
//         self.1.bool(value) self.0.bool()
//     }

//     fn char(self) -> Result<char, Self::Error> {
//         todo!()
//     }

//     fn u8(self) -> Result<u8, Self::Error> {
//         todo!()
//     }

//     fn u16(self) -> Result<u16, Self::Error> {
//         todo!()
//     }

//     fn u32(self) -> Result<u32, Self::Error> {
//         todo!()
//     }

//     fn u64(self) -> Result<u64, Self::Error> {
//         todo!()
//     }

//     fn u128(self) -> Result<u128, Self::Error> {
//         todo!()
//     }

//     fn usize(self) -> Result<usize, Self::Error> {
//         todo!()
//     }

//     fn i8(self) -> Result<i8, Self::Error> {
//         todo!()
//     }

//     fn i16(self) -> Result<i16, Self::Error> {
//         todo!()
//     }

//     fn i32(self) -> Result<i32, Self::Error> {
//         todo!()
//     }

//     fn i64(self) -> Result<i64, Self::Error> {
//         todo!()
//     }

//     fn i128(self) -> Result<i128, Self::Error> {
//         todo!()
//     }

//     fn isize(self) -> Result<isize, Self::Error> {
//         todo!()
//     }

//     fn f32(self) -> Result<f32, Self::Error> {
//         todo!()
//     }

//     fn f64(self) -> Result<f64, Self::Error> {
//         todo!()
//     }
// }
