pub trait FullIterator: Iterator + DoubleEndedIterator + ExactSizeIterator {}
impl<I: Iterator + DoubleEndedIterator + ExactSizeIterator> FullIterator for I {}
