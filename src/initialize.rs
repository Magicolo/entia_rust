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

entia_macro::recurse_32!(initialize);
