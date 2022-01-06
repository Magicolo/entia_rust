use entia_check::*;

fn main() {
    trait Boba {
        type Fett: Boba;
        fn fett(self) -> Option<Self::Fett>;
    }

    impl Boba for usize {
        type Fett = Option<isize>;
        fn fett(self) -> Option<Self::Fett> {
            None
        }
    }

    impl Boba for isize {
        type Fett = Option<usize>;
        fn fett(self) -> Option<Self::Fett> {
            None
        }
    }

    impl<B: Boba> Boba for Option<B> {
        type Fett = B::Fett;
        fn fett(self) -> Option<Self::Fett> {
            None
        }
    }

    fn boba<B: Boba>(value: B, count: usize) -> usize {
        if count > 0 {
            boba(value.fett(), count - 1)
        } else {
            count
        }
    }

    println!("{}", boba(1usize, 100));

    let result = &u8::generator().map(|value| value).check(1000, |_| true);
    // let result = check(&usize::generator().collect::<Vec<_>>(), 1000, |item| {
    //     item.len() < 100
    // });
    println!("{:?}", result);
}
