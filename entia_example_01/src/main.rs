use std::time::{Duration, Instant};

use entia::*;
use winput::message_loop::Event;
use winput::{message_loop, Action, Vk};

#[derive(Clone, Debug)]
enum Input {
    Left(bool),
    Right(bool),
    Down(bool),
    Up(bool),
}
impl Message for Input {}

#[derive(Copy, Clone, Debug)]
struct Position(isize, isize);
impl Component for Position {}

#[derive(Copy, Clone, Debug)]
struct Controller;
impl Component for Controller {}

#[derive(Default)]
struct Time {
    pub frames: usize,
    pub total: Duration,
    pub delta: Duration,
}
impl Resource for Time {}

fn main() {
    fn run() -> Result<(), Error> {
        let mut world = World::new();

        world.run(|mut create: Create<_>| {
            create.one(spawn(spawn(spawn((Position(0, 0), Controller)))));
        })?;

        let mut runner = world.scheduler().pipe(time).pipe(input).runner()?;
        loop {
            runner = runner.run(&mut world)?;
        }
    }

    run().unwrap();
}

fn time(scheduler: Scheduler) -> Scheduler {
    let start = Instant::now();
    scheduler.schedule(move |time: &mut Time| {
        let total = Instant::now().duration_since(start);
        *time = Time {
            frames: time.frames + 1,
            delta: total - time.total,
            total,
        };
    })
}

fn input(scheduler: Scheduler) -> Scheduler {
    let events = message_loop::start().unwrap();
    scheduler
        .schedule(move |mut inputs: Emit<Input>| {
            while let Some(event) = events.try_next_event() {
                match event {
                    Event::Keyboard {
                        vk: Vk::LeftArrow,
                        action,
                        ..
                    } => inputs.emit(Input::Left(action == Action::Press)),
                    Event::Keyboard {
                        vk: Vk::RightArrow,
                        action,
                        ..
                    } => inputs.emit(Input::Right(action == Action::Press)),
                    Event::Keyboard {
                        vk: Vk::UpArrow,
                        action,
                        ..
                    } => inputs.emit(Input::Up(action == Action::Press)),
                    Event::Keyboard {
                        vk: Vk::DownArrow,
                        action,
                        ..
                    } => inputs.emit(Input::Down(action == Action::Press)),
                    _ => {}
                }
            }
        })
        .schedule(
            |inputs: Receive<Input>,
             entities: &Entities,
             query: Query<(&mut Position, Family, Entity, Parent<Entity>), Controller>| {
                for input in inputs {
                    match input {
                        Input::Left(true) => query.each(|(position, ..)| position.0 -= 1),
                        Input::Right(true) => query.each(|(position, ..)| position.0 += 1),
                        Input::Down(true) => query.each(|(position, ..)| position.1 -= 1),
                        Input::Up(true) => query.each(|(position, ..)| position.1 += 1),
                        _ => {}
                    }

                    // TODO
                    // for (_, entity, parent) in &query {
                    //     if let Some(parent) = parent.get(0) {
                    //         entities.reject(entity, parent);
                    //     }
                    // }

                    println!(
                        "INPUT: {:?} \n {:?} \n {:?} \n {:?} \n {:?}",
                        input,
                        query.into_iter().map(|(position, ..)| position).collect::<Vec<_>>(),
                        query
                            .into_iter()
                            .map(|(.., entity, _)| (
                                entity,
                                entities.root(entity),
                                entities.parent(entity),
                                entities
                                    .ancestors(entity, Vertical::FromBottom)
                                    .collect::<Vec<_>>(),
                            ))
                            .collect::<Vec<_>>(),
                        query
                            .into_iter()
                            .map(|(.., family, _, _)| (
                                family.entity(),
                                family.parent(),
                                family.ancestors(Vertical::FromBottom).collect::<Vec<_>>(),
                            ))
                            .collect::<Vec<_>>(),
                        query
                            .into_iter()
                            .map(|(.., entity, parent)| (
                                parent.get(0),
                                parent.get(1),
                                parent.get(2),
                                parent.get(3),
                                parent.into_iter().collect::<Vec<_>>(),
                                parent.into_iter().last().unwrap_or(entity)
                            ))
                            .collect::<Vec<_>>(),
                    );
                }
            },
        )
}
