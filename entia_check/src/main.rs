use entia_check::{primitive::Full, *};

fn main() {
    let items: Vec<_> = Full::<u16>::default().sample(100).collect();
    println!("{:?}", items);
}
