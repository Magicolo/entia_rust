use entia::*;
use piston::WindowSettings;
use piston_window::*;
use std::{error, time::Duration};

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

// TODO: Allow input-output systems...
// TODO: More aggressive parallelization of systems.
// - Try to bundle systems after a conflict?

fn main() {
    run().unwrap();
}

fn run() -> Result<(), Box<dyn error::Error>> {
    let mut world = World::new();
    let mut window: PistonWindow = WindowSettings::new("Example_01", [640, 480])
        .exit_on_esc(true)
        .build()?;

    let mut initialize = world
        .scheduler()
        .add(|mut create: Create<_>| {
            create.clones(spawn(spawn(spawn((Position(0, 0), Controller)))), 5);
        })
        .schedule()?;

    let mut inputs = world.injector::<Emit<Input>>()?;
    let mut time = world.injector::<&mut Time>()?;
    let mut update = world.scheduler().add(apply_input_to_position).schedule()?;

    initialize.run(&mut world)?;
    while let Some(event) = window.next() {
        // window
        //     .draw_2d(&event, |a, b, c| render.run(&mut world))
        //     .unwrap_or(Ok(()))?;
        match event {
            Event::Input(piston::Input::Button(ButtonArgs { state, button, .. }), _) => {
                let mut guard = inputs.guard(&mut world)?;
                let mut inputs = guard.inject();
                let press = state == ButtonState::Press;
                match button {
                    Button::Keyboard(Key::Left) => inputs.emit(Input::Left(press)),
                    Button::Keyboard(Key::Right) => inputs.emit(Input::Right(press)),
                    Button::Keyboard(Key::Down) => inputs.emit(Input::Down(press)),
                    Button::Keyboard(Key::Up) => inputs.emit(Input::Up(press)),
                    _ => {}
                }
            }
            Event::Loop(Loop::Update(UpdateArgs { dt })) => {
                {
                    let mut guard = time.guard(&mut world)?;
                    let time = guard.inject();
                    time.frames += 1;
                    time.delta = Duration::from_secs_f64(dt);
                    time.total += time.delta;
                }
                update.run(&mut world)?
            }
            Event::Loop(Loop::Render(_)) => {}
            _ => {}
        }
    }

    Ok(())
}

fn apply_input_to_position(inputs: Receive<Input>, query: Query<&mut Position, Controller>) {
    for input in inputs {
        match input {
            Input::Left(true) => query.each(|position| position.0 -= 1),
            Input::Right(true) => query.each(|position| position.0 += 1),
            Input::Down(true) => query.each(|position| position.1 -= 1),
            Input::Up(true) => query.each(|position| position.1 += 1),
            _ => {}
        }

        println!("{:?}", input);
    }
}
