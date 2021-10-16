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

struct Position(usize, usize);
impl Component for Position {}

struct Controller;

fn main() {
    let mut world = World::new();
    let mut runner = world.scheduler().pipe(input).runner().unwrap();
    loop {
        runner.run(&mut world);
    }
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
        .schedule(|inputs: Receive<Input>| {
            for input in inputs {
                println!("INPUT: {:?}", input);
            }
        })
        .schedule(|input: Receive<Input>| for input in input {})
}
