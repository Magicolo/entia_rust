use entia_check::*;

fn main() {
    for pair in <(f32, f32)>::generator().sample(100) {
        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
        if low == high {
            continue;
        }
        assert!((low..high)
            .sample(100)
            .all(|value| value >= low && value < high));
    }
}
