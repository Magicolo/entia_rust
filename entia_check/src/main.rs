use entia_check::*;

fn main() {
    const COUNT: usize = 5000;
    type T = f32;

    for high in <T>::generator().sample(COUNT).filter(|a| a.is_finite()) {
        for value in (..=high)
            .generator()
            .sample(COUNT)
            .filter(|a| a.is_finite())
        {
            assert!(value <= high);
        }
    }
}
