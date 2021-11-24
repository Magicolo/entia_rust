use crate::query::item::Item;

use super::{error::Result, *};

#[test]
fn resource_read_read_aa() -> Result {
    inject::<(&Time, &Time)>()?;
    inject::<(&Physics, &Physics)>()?;
    Ok(())
}

#[test]
fn resource_read_read_ab() -> Result {
    inject::<(&Time, &Physics)>()?;
    inject::<(&Physics, &Time)>()?;
    Ok(())
}

#[test]
fn resource_read_read_aabb() -> Result {
    inject::<(&Time, &Time, &Physics, &Physics)>()?;
    inject::<(&Physics, &Physics, &Time, &Time)>()?;
    Ok(())
}

#[test]
#[should_panic]
fn resource_read_write() {
    inject::<(&Time, &mut Time)>().unwrap()
}

#[test]
#[should_panic]
fn resource_write_read() {
    inject::<(&mut Time, &Time)>().unwrap()
}

#[test]
#[should_panic]
fn resource_write_write() {
    inject::<(&mut Time, &mut Time)>().unwrap()
}

#[allow(type_alias_bounds)]
type CreateQuery<'a, I: Item> = (
    Create<'a, Add<Position>>,
    Create<'a, Add<Velocity>>,
    Create<'a, (Add<Position>, Add<Velocity>)>,
    Query<'a, I>,
);

#[test]
fn query_read_read_aa() -> Result {
    inject::<CreateQuery<(&Position, &Position)>>()?;
    inject::<CreateQuery<(&Velocity, &Velocity)>>()?;
    Ok(())
}

#[test]
fn query_read_read_ab() -> Result {
    inject::<CreateQuery<(&Position, &Velocity)>>()?;
    inject::<CreateQuery<(&Velocity, &Position)>>()?;
    Ok(())
}

#[test]
fn query_read_read_aa_bb() -> Result {
    inject::<CreateQuery<(&Position, &Position, &Velocity, &Velocity)>>()?;
    inject::<CreateQuery<(&Velocity, &Velocity, &Position, &Position)>>()?;
    Ok(())
}

#[test]
#[should_panic]
fn query_read_write() {
    inject::<CreateQuery<(&Position, &mut Position)>>().unwrap();
}

#[test]
#[should_panic]
fn query_write_read() {
    inject::<CreateQuery<(&mut Position, &Position)>>().unwrap();
}

#[test]
#[should_panic]
fn query_write_write() {
    inject::<CreateQuery<(&mut Position, &mut Position)>>().unwrap();
}
