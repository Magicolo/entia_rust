use entia_check::*;

fn main() {
    let items: Vec<_> = usize::generator().sample(100).collect();
    println!("{:?}", items);
}
