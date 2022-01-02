use entia_check::*;

fn main() {
    let items = <f64>::generator().sample(256).collect::<Vec<_>>();
    // let items = positive::<i32>().sample(100).collect::<Vec<_>>();
    println!("{:?}", items);

    // for pair in <(f32, f32)>::generator()
    //     .sample(1000)
    //     .filter(|(low, high)| low.is_finite() && high.is_finite())
    // {
    //     let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
    //     if high - low < f32::EPSILON {
    //         continue;
    //     }
    //     for value in (low..high)
    //         .generator()
    //         .sample(1000)
    //         .filter(|value| value.is_finite())
    //     {
    //         assert!(value >= low);
    //         assert!(value < high);
    //     }
    // }
}
