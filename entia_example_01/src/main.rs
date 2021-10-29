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

#[derive(Copy, Clone, Debug)]
struct Position(isize, isize);

#[derive(Copy, Clone, Debug)]
struct Render {
    color: [f32; 4],
    visible: bool,
}

#[derive(Copy, Clone, Debug)]
struct Controller;

#[derive(Default)]
struct Time {
    pub frames: usize,
    pub total: Duration,
    pub delta: Duration,
}

// TODO: More aggressive parallelization of systems.
// - Try to bundle systems after a conflict?
// TODO: Derive macros for 'Inject', 'Item', 'Initial', 'Filter'

fn main() {
    run().unwrap();
}

fn run() -> Result<(), Box<dyn error::Error>> {
    const SIZE: [f64; 2] = [640., 480.];
    const CELLS: [f64; 2] = [10., 10.];

    let mut world = World::new();
    let mut window: PistonWindow = WindowSettings::new("Example_01", SIZE)
        .exit_on_esc(true)
        .build()?;

    let mut initialize = world
        .scheduler()
        .add(|mut create: Create<_>| {
            create.all((0..10).map(|i| {
                (
                    add(Position(i, -i)),
                    add(Render {
                        color: [0.1, 0.1, 0.8, 1.],
                        visible: true,
                    }),
                    add(Controller),
                )
            }));
        })
        .schedule()?;

    let mut inputs = world.injector::<Emit<Input>>()?;
    let mut time = world.injector::<&mut Time>()?;
    let mut render = world.injector::<Query<(&Position, &Render)>>()?;
    let mut runner = world.scheduler().add(apply_input_to_position).schedule()?;

    initialize.run(&mut world)?;
    while let Some(event) = window.next() {
        match event {
            Event::Input(
                piston::Input::Button(ButtonArgs {
                    state,
                    button: Button::Keyboard(key),
                    ..
                }),
                _,
            ) => {
                let mut guard = inputs.guard(&mut world)?;
                let mut inputs = guard.inject();
                let press = state == ButtonState::Press;
                match key {
                    Key::Left => inputs.emit(Input::Left(press)),
                    Key::Right => inputs.emit(Input::Right(press)),
                    Key::Down => inputs.emit(Input::Down(press)),
                    Key::Up => inputs.emit(Input::Up(press)),
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
                runner.run(&mut world)?
            }
            event => window
                .draw_2d(&event, |context, graphics, _| -> Result<(), Error> {
                    graphics.clear_color([0.25, 0.4, 0.1, 1.]);

                    let cell_size = [SIZE[0] / CELLS[0], SIZE[1] / CELLS[1]];
                    let square = rectangle::square(0., 0., cell_size[1]);
                    let mut guard = render.guard(&mut world)?;
                    for (position, render) in &guard.inject() {
                        if render.visible {
                            let x = position.0.rem_euclid(CELLS[0] as isize) as f64 * cell_size[0];
                            let y =
                                (-position.1).rem_euclid(CELLS[1] as isize) as f64 * cell_size[1];
                            let transform = context.transform.trans(x, y);
                            Rectangle::new(render.color).draw(
                                square,
                                &context.draw_state,
                                transform,
                                graphics,
                            );
                        }
                    }
                    Ok(())
                })
                .unwrap_or(Ok(()))?,
        }
    }

    Ok(())
}

fn apply_input_to_position(inputs: Receive<Input>, query: Query<&mut Position, Has<Controller>>) {
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
