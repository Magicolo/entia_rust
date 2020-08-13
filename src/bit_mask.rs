use std::mem;

pub struct Bucket {
    index: usize,
    mask: usize,
}

pub struct BitMask {
    buckets: Vec<usize>,
}

impl Bucket {
    const SIZE: usize = mem::size_of::<usize>();

    pub const fn new(index: usize, mask: usize) -> Self {
        Self { index, mask }
    }

    #[inline]
    pub const fn from_bit(index: usize) -> Self {
        Self::new(index / Self::SIZE, 1 << (index % Self::SIZE))
    }
}

impl BitMask {
    #[inline]
    pub fn has_bit(&self, index: usize) -> bool {
        self.has_bucket(&Bucket::from_bit(index))
    }

    #[inline]
    pub fn has_bucket(&self, bucket: &Bucket) -> bool {
        self.buckets
            .get(bucket.index)
            .map_or(false, |mask| (mask & bucket.mask) == bucket.mask)
    }

    // pub fn has_all(&self, mask: BitMask) -> bool {}
    // pub fn has_any(&self, mask: BitMask) -> bool {}
    // pub fn has_none(&self, mask: BitMask) -> bool {}
}
