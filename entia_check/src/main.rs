use entia_check::{generator::State, *};
use std::fmt;

fn main() {
    trait Boba {
        type Fett: Boba;
        fn fett(self) -> Option<Self::Fett>;
    }

    impl Boba for usize {
        type Fett = Option<isize>;
        fn fett(self) -> Option<Self::Fett> {
            None
        }
    }

    impl Boba for isize {
        type Fett = Option<usize>;
        fn fett(self) -> Option<Self::Fett> {
            None
        }
    }

    impl<B: Boba> Boba for Option<B> {
        type Fett = B::Fett;
        fn fett(self) -> Option<Self::Fett> {
            None
        }
    }

    fn boba<B: Boba>(value: B, count: usize) -> usize {
        if count > 0 {
            boba(value.fett(), count - 1)
        } else {
            count
        }
    }

    println!("{}", boba(1usize, 100));

    let result = check(&u8::generator(), 1000, |&item| item < 100);
    // let result = check(&usize::generator().collect::<Vec<_>>(), 1000, |item| {
    //     item.len() < 100
    // });
    println!("{:?}", result);

    fn check<G: Generator<Item = impl fmt::Debug>, V: Fn(&G::Item) -> bool>(
        generator: &G,
        count: usize,
        valid: V,
    ) -> Result<(), G::Item> {
        // TODO: Parallelize checking!
        for mut state in State::new(count) {
            let old = state.clone();
            let pair = generator.generate(&mut state);
            if !valid(&pair.0) {
                println!("Begin shrink: {:?}", pair.0);
                let item = shrink(generator, pair, old, valid);
                println!("End shrink: {:?}", item);
                return Err(item);
            }
        }
        Ok(())
    }

    fn shrink<G: Generator, V: Fn(&G::Item) -> bool>(
        generator: &G,
        mut pair: (G::Item, G::State),
        state: State,
        valid: V,
    ) -> G::Item {
        while let Some(generator) = generator.shrink(&mut pair.1) {
            let pair = generator.generate(&mut state.clone());
            if valid(&pair.0) {
                continue;
            } else {
                return shrink(&generator, pair, state, valid);
            }
        }
        pair.0
    }
}
