pub trait Initialize {
    // fn initialize(world: &mut World);
}

macro_rules! initialize {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Send + 'static,)*> Initialize for ($($t,)*) {
            // fn initialize(world: &mut World) {
            //     $($t::initialize(world);)*
            // }
            // fn metas(_world: &mut World) -> Vec<Meta> {
            //     vec![$(_world.get_or_add_meta::<$t>(),)*]
            // }
        }
    };
}

crate::recurse!(
    initialize, input0, T0, input1, T1, input2, T2, input3, T3, input4, T4, input5, T5, input6, T6,
    input7, T7, input8, T8, input9, T9, input10, T10, input11, T11, input12, T12, input13, T13,
    input14, T14, input15, T15
);
