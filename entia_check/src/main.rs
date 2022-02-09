use entia_check::*;

fn main() {
    // let result = bool::generator().collect().check(1000, |values: &Vec<_>| {
    //     values.iter().filter(|&&value| value).count() < 9
    // });
    // println!("{:?}", result);
    let result = <(u8, u8)>::generator().check(1000, |&(a, b)| a < 100);
    println!("{:?}", result);
}
