#[macro_export]
macro_rules! ignore {
    ($($i:tt)*) => {};
}

#[macro_export]
macro_rules! count {
    () => { 0 };
    ($v:ident $(,$vs:ident)*) => {1 + $crate::count!($($vs),*) };
}

#[macro_export]
macro_rules! first {
    () => {};
    ($v:ident $(,$vs:ident)*) => {
        $v
    };
}

#[macro_export]
macro_rules! last {
    () => {};
    ($($vs:ident,)* $v:ident) => {
        $v
    };
}

#[macro_export]
macro_rules! tuples_8 {
    ($m:ident) => {
        $m!();
        $m!(p0, T0);
        $m!(p0, T0, p1, T1);
        $m!(p0, T0, p1, T1, p2, T2);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7);
    };
}

#[macro_export]
macro_rules! tuples_16 {
    ($m:ident) => {
        $m!();
        $m!(p0, T0);
        $m!(p0, T0, p1, T1);
        $m!(p0, T0, p1, T1, p2, T2);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9);
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15
        );
    };
}

#[macro_export]
macro_rules! tuples_32 {
    ($m:ident) => {
        $m!();
        $m!(p0, T0);
        $m!(p0, T0, p1, T1);
        $m!(p0, T0, p1, T1, p2, T2);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8);
        $m!(p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9);
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21, p22, T22
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21, p22, T22, p23, T23
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24, p25, T25
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26, p27,
            T27
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26, p27,
            T27, p28, T28
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26, p27,
            T27, p28, T28, p29, T29
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26, p27,
            T27, p28, T28, p29, T29, p30, T30
        );
        $m!(
            p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9, p10,
            T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18, T18,
            p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26, p27,
            T27, p28, T28, p29, T29, p30, T30, p31, T31
        );
    };
}

#[macro_export]
macro_rules! tuples_with_8 {
    ($m:ident) => {
        $m!(Tuples0, 0);
        $m!(Tuples1, 1, p0, T0, 0);
        $m!(Tuples2, 2, p0, T0, 0, p1, T1, 1);
        $m!(Tuples3, 3, p0, T0, 0, p1, T1, 1, p2, T2, 2);
        $m!(Tuples4, 4, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3);
        $m!(Tuples5, 5, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4);
        $m!(Tuples6, 6, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5);
        $m!(
            Tuples7, 7, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6
        );
        $m!(
            Tuples8, 8, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7
        );
    };
}

#[macro_export]
macro_rules! tuples_with_16 {
    ($m:ident) => {
        $m!(Tuples0, 0);
        $m!(Tuples1, 1, p0, T0, 0);
        $m!(Tuples2, 2, p0, T0, 0, p1, T1, 1);
        $m!(Tuples3, 3, p0, T0, 0, p1, T1, 1, p2, T2, 2);
        $m!(Tuples4, 4, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3);
        $m!(Tuples5, 5, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4);
        $m!(Tuples6, 6, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5);
        $m!(
            Tuples7, 7, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6
        );
        $m!(
            Tuples8, 8, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7
        );
        $m!(
            Tuples9, 9, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8
        );
        $m!(
            Tuples10, 10, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9
        );
        $m!(
            Tuples11, 11, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10
        );
        $m!(
            Tuples12, 12, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11
        );
        $m!(
            Tuples13, 13, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12
        );
        $m!(
            Tuples14, 14, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13
        );
        $m!(
            Tuples15, 15, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14
        );
        $m!(
            Tuples16, 16, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15
        );
    };
}

#[macro_export]
macro_rules! tuples_with_32 {
    ($m:ident) => {
        $m!(Tuples0, 0);
        $m!(Tuples1, 1, p0, T0, 0);
        $m!(Tuples2, 2, p0, T0, 0, p1, T1, 1);
        $m!(Tuples3, 3, p0, T0, 0, p1, T1, 1, p2, T2, 2);
        $m!(Tuples4, 4, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3);
        $m!(Tuples5, 5, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4);
        $m!(Tuples6, 6, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5);
        $m!(
            Tuples7, 7, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6, 6
        );
        $m!(
            Tuples8, 8, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7
        );
        $m!(
            Tuples9, 9, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8
        );
        $m!(
            Tuples10, 10, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9
        );
        $m!(
            Tuples11, 11, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10
        );
        $m!(
            Tuples12, 12, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11
        );
        $m!(
            Tuples13, 13, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12
        );
        $m!(
            Tuples14, 14, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13
        );
        $m!(
            Tuples15, 15, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14
        );
        $m!(
            Tuples16, 16, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15
        );
        $m!(
            Tuples17, 17, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16
        );
        $m!(
            Tuples18, 18, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17
        );
        $m!(
            Tuples19, 19, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18
        );
        $m!(
            Tuples20, 20, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19
        );
        $m!(
            Tuples21, 21, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20
        );
        $m!(
            Tuples22, 22, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21
        );
        $m!(
            Tuples23, 23, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21, p22, T22, 22
        );
        $m!(
            Tuples24, 24, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21, p22, T22, 22, p23, T23, 23
        );
        $m!(
            Tuples25, 25, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21, p22, T22, 22, p23, T23, 23, p24, T24, 24
        );
        $m!(
            Tuples26, 26, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21, p22, T22, 22, p23, T23, 23, p24, T24, 24, p25, T25, 25
        );
        $m!(
            Tuples27, 27, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21, p22, T22, 22, p23, T23, 23, p24, T24, 24, p25, T25, 25,
            p26, T26, 26
        );
        $m!(
            Tuples28, 28, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21, p22, T22, 22, p23, T23, 23, p24, T24, 24, p25, T25, 25,
            p26, T26, 26, p27, T27, 27
        );
        $m!(
            Tuples29, 29, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21, p22, T22, 22, p23, T23, 23, p24, T24, 24, p25, T25, 25,
            p26, T26, 26, p27, T27, 27, p28, T28, 28
        );
        $m!(
            Tuples30, 30, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21, p22, T22, 22, p23, T23, 23, p24, T24, 24, p25, T25, 25,
            p26, T26, 26, p27, T27, 27, p28, T28, 28, p29, T29, 29
        );
        $m!(
            Tuples31, 31, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21, p22, T22, 22, p23, T23, 23, p24, T24, 24, p25, T25, 25,
            p26, T26, 26, p27, T27, 27, p28, T28, 28, p29, T29, 29, p30, T30, 30
        );
        $m!(
            Tuples32, 32, p0, T0, 0, p1, T1, 1, p2, T2, 2, p3, T3, 3, p4, T4, 4, p5, T5, 5, p6, T6,
            6, p7, T7, 7, p8, T8, 8, p9, T9, 9, p10, T10, 10, p11, T11, 11, p12, T12, 12, p13, T13,
            13, p14, T14, 14, p15, T15, 15, p16, T16, 16, p17, T17, 17, p18, T18, 18, p19, T19, 19,
            p20, T20, 20, p21, T21, 21, p22, T22, 22, p23, T23, 23, p24, T24, 24, p25, T25, 25,
            p26, T26, 26, p27, T27, 27, p28, T28, 28, p29, T29, 29, p30, T30, 30, p31, T31, 31
        );
    };
}
