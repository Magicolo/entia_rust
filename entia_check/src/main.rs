use entia_check::*;

fn main() {
    assert!(string(digit()).sample(1000).all(|value| {
        println!("{}", value);
        value.is_ascii()
    }));
}
