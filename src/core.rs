pub trait Change {
    fn change(&mut self, value: Self) -> bool;
}

impl<T: PartialEq> Change for T {
    fn change(&mut self, value: Self) -> bool {
        if *self == value {
            false
        } else {
            *self = value;
            true
        }
    }
}
