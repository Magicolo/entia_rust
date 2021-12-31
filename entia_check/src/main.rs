mod all;
mod any;
mod generator;
mod primitive;

use generator::*;

fn main() {
    for pair in <(f32, f32)>::generator().sample(1000) {
        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
        if low == high {
            continue;
        }

        for value in (low..high)
            .generator()
            .sample(10000)
            .filter(|value| value.is_finite())
        {
            assert!(value >= low && value <= high);
        }
    }
}
