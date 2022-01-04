use entia_check::{generator::State, *};

fn main() {
    let mut generator = usize::generator().collect::<Vec<_>>();
    for mut state in State::new(1000) {
        let items = generator.generate(&mut state);
        if items.len() < 100 {
            continue;
        }

        println!("Try to shrink: {:?}", items);
        println!("Shrink: {:?}", items);
        println!("Shrinked to: {:?}", items);
    }

    fn check<G: Generator>(
        generator: &mut G,
        state: &mut State,
        mut valid: impl FnMut(&G::Item) -> bool,
    ) -> Result<G::Item, G::Item> {
        let mut current = state.clone();
        let mut item = generator.generate(&mut current);
        if valid(&item) {
            *state = current;
            return Ok(item);
        } else {
            while let Some(mut generator) = generator.shrink() {
                let mut current_state = state.clone();
                let current_item = generator.generate(state);
                if valid(&current_item) {
                    continue;
                } else {
                    return check(&mut generator, state, valid);
                }
            }
            Err(item)
        }

        // else if let Some(mut generator) = generator.shrink() {
        //     // Use the original state.
        //     check(&mut generator, state, valid)
        // } else {
        //     *state = current;
        //     Ok(item)
        // }
    }
}
