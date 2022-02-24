use entia_check::*;

fn main() {
    // let result = bool::generator().collect().check(1000, |values: &Vec<_>| {
    //     values.len() < 10
    //     // values.iter().filter(|&&value| value).count() < 9
    // });
    // let result = u8::generator().check(1000, |&a| a < 100);
    // let result = (i8::MIN.., ..i8::MAX)
    //     .generator()
    //     .check(1000, |&(a, _)| a < 100);

    // TODO: fix shrink forever
    let result = <(usize, usize)>::generator().check(1000, None, |&(left, right)| left <= right);
    println!("{:?}", result);
    let result = <(usize, usize)>::generator().check(1000, None, |&(left, right)| left >= right);
    println!("{:?}", result);

    let generate = i32::generator();
    for right in generate.sample(1000) {
        if right >= 0 {
            match generate.check(1000, None, |&left| left <= right) {
                Ok(_) => println!("{}", right),
                Err(_) => {} //assert_eq!(*report.shrunk(), right)}, //println!("{:?} | {}", report, right),
            }
        } else {
            match generate.check(1000, None, |&left| left >= right) {
                Ok(_) => println!("{}", right),
                Err(_) => {} // assert_eq!(*report.shrunk(), right), //println!("{:?} | {}", report, right),
            }
        }
    }
    // let result = isize::generator().check(1000, |&value| value < 12345678987654321);
    // println!("{:?}", result);
}
