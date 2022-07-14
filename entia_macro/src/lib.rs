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
macro_rules! recurse {
    ($m:ident) => { $m!(); };
    ($m:ident, $p:ident, $t:ident $(,$ps:ident, $ts:ident)* $(,)?) => {
        $m!($p, $t $(,$ps, $ts)*);
        $crate::recurse!($m $(,$ps, $ts)*);
    };
}

#[macro_export]
macro_rules! recurse_8 {
    ($m:ident) => {
        $crate::recurse!($m, p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7);
    };
}

#[macro_export]
macro_rules! recurse_16 {
    ($m:ident) => {
        $crate::recurse!(
            $m, p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9,
            p10, T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15
        );
    };
}

#[macro_export]
macro_rules! recurse_32 {
    ($m:ident) => {
        $crate::recurse!(
            $m, p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9,
            p10, T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18,
            T18, p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26,
            p27, T27, p28, T28, p29, T29, p30, T30, p31, T31
        );
    };
}

#[macro_export]
macro_rules! recurse_64 {
    ($m:ident) => {
        $crate::recurse!(
            $m, p0, T0, p1, T1, p2, T2, p3, T3, p4, T4, p5, T5, p6, T6, p7, T7, p8, T8, p9, T9,
            p10, T10, p11, T11, p12, T12, p13, T13, p14, T14, p15, T15, p16, T16, p17, T17, p18,
            T18, p19, T19, p20, T20, p21, T21, p22, T22, p23, T23, p24, T24, p25, T25, p26, T26,
            p27, T27, p28, T28, p29, T29, p30, T30, p31, T31, p32, T32, p33, T33, p34, T34, p35,
            T35, p36, T36, p37, T37, p38, T38, p39, T39, p40, T40, p41, T41, p42, T42, p43, T43,
            p44, T44, p45, T45, p46, T46, p47, T47, p48, T48, p49, T49, p50, T50, p51, T51, p52,
            T52, p53, T53, p54, T54, p55, T55, p56, T56, p57, T57, p58, T58, p59, T59, p60, T60,
            p61, T61, p62, T62, p63, T63
        );
    };
}
