pub mod all;
pub mod any;
pub mod generator;
pub mod primitive;

#[cfg(test)]
mod test {
    use crate::{any::Any, generator::*};

    #[test]
    fn boba() {
        let mut state = State::default();
        let _a = [1, 2].generate(&mut state);
        let _a = ('a'..='b').generate(&mut state);
        let _a = <()>::generate(&mut state);
        let _a = <(char, u8)>::generate(&mut state);
        let _a = <(i32, u128)>::generate(&mut state);
        let _a = <(usize,)>::generate(&mut state);
        let _a = Any::<(bool, bool)>::generate(&mut state);
        let _a = Any::<(isize,)>::generate(&mut state);
        let _a = Vec::<(u16, i64)>::generate(&mut state);
        let _generator = Generator::map(0..100usize, |count| vec![1; count]);
    }
}
