use entia_check::{generator::State, *};
use std::fmt;

fn main() {
    let result = check(&usize::generator().collect::<Vec<_>>(), 1000, |item| {
        item.len() < 100
    });
    println!("{:?}", result);

    fn check<G: Generator<Item = impl fmt::Debug>>(
        generator: &G,
        count: usize,
        valid: impl Fn(&G::Item) -> bool,
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

    fn shrink<G: Generator>(
        generator: &G,
        mut pair: (G::Item, G::State),
        state: State,
        mut valid: impl FnMut(&G::Item) -> bool,
    ) -> G::Item {
        while let Some(mut generator) = generator.shrink(&mut pair.1) {
            let pair = generator.generate(&mut state.clone());
            if valid(&pair.0) {
                continue;
            } else {
                return shrink(&mut generator, pair, state, valid);
            }
        }
        pair.0
    }
}
