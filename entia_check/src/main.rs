use all::All;
use entia_check::*;

fn main() {
    let result = All::from([u8::generator()]).check(1000, |_| true);
    println!("{:?}", result);
}
