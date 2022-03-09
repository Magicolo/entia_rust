use entia_check::*;

fn main() {
    (
        number::<u8>(),
        number::<u8>(),
        number::<u8>(),
        number::<u8>(),
    )
        .bind(|(left, a, b, c)| (left, a, b, c, number::<u8>()))
        .check(1000, |&(left, .., right)| left < 100 || left < right)
        .unwrap();
}
