// TODO: Allows to wrap a type 'T' and replace its dependencies with the dependencies of type 'D'.
pub struct Depend<T, D>(T);

// TODO: This trait should be implemented by all 'Inject/Item/Modify' implementations.
pub trait Dependencies {
    fn depend(&self) -> Vec<Dependency>;
}
