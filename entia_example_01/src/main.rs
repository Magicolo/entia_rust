use entia::{error::Error, *};
use piston::WindowSettings;
use piston_window::*;
use std::{collections::VecDeque, error, result::Result, time::Duration};

/*
STATIC AND DYNAMIC COMPONENTS:
    - The 'Item' trait will need an 'static_at' and a 'dynamic_at' where the difference will be that a 'dynamic_at' returns an 'Option<T>'.
    - A query will know to use the 'static_at' or the 'dynamic_at' when iterating by checking if all the stores it uses are 'static'.
        - This can be checked in 'update' if a new 'dynamic' component increases the 'world.version'.
    - There might be a way to make this quite granular by allowing individual stores to change from 'static' to 'dynamic' and back.


BUGS:
    - Some dependencies may work weirdly with 'Entity' as a component. If it cannot be reconciled, prevent 'Entity' as a component.

TODO:
    - Add a way to iterate a query in parallel.
        - A 'mut' access to query items should be allowed since the 'ChunkIterator' will prevent aliasing a shared pointer.
    - Add tests.
        - The query 'Query<&mut Entity>' should be valid but should not allow modifying the 'entity_store'.
        - The resource '&mut Entity' should be valid but should not modify any segment store.
    - Review all dependencies.
    - Reorganize 'pub use'.
        - Inject modules should be grouped together.
    - Reorganize 'error::Error'.
        - Use subgroups to allow more specific errors (ex: Error::Duplicate(duplicate::Error)).
    - Is it possible to extract a (serializable) template from an entity?
    - Is it possible to copy an entity's components to another entity?
    - Prevent 'Query<&Entity>' and 'Query<&Family>' since they are confusing and, in the case of '&Family' will result
    in a suprising failure.
    - Currently, using 'world.set_meta' will not update current meta users (including segments). This might be a acceptable behavior.
    - How to serialize an entity with all its (serializable) components?
    - Think about 'query::item::child' some more.
    - What about these (might require the 'At' trait to return an 'Option<Self::Item>')?
        - 'Child<I, F>'
            - get the first child that matches
            - fails if no match is found
        - 'Children<I, F>'
            - gets all children that match
            - never fails
        - 'Ancestor<I, F>'
        - 'Ancestors<I, F>'
        - 'Descendant<I, F>'
        - 'Descendants<I, F>'
        - 'Root<I, F>'
        If the 'At' trait returns an 'Option', it allows for dynamic filters as well.
            - The 'Filter' trait would have 2 methods: 'static_filter' and 'dynamic_filter'.
*/

/*
Coherence rules:
- Within one system, structural operations will not be visible until resolution.
For example, it is ok for any query to overlap with a 'Create' operation, but to ensure coherence,
the newly created entities will not be added to the query until the system has completed its execution.
- Between systems, structural operations will only force an ordering if an operation could require the result of a
previous one.
For example, if system A has 'Add' and system B has 'Remove', they will be ordered by the system declaration order
since after the execution of both systems, the presence or not of components that the 'Add/Remove' modify must be
deterministic.
- Create: Since other threads don't have access to newly created entities other than through a 'Write' dependency
(which incurs an ordering), it is compatible with other defered operation.

________| Create  | Destroy | Add     | Remove  | Read    | Write   |
Create  |    Y    |    Y    |    Y    |    Y    |    N    |    N    |
Destroy |   Y*1   |    Y    |   Y*2   |   Y*2   |    N    |    N    |
Add     |   Y*1   |    Y    |   Y*3   |    Y    |    N    |    N    |
Remove  |   Y*1   |    Y    |    Y    |    Y    |    N    |    N    |
Read    |    Y    |    Y    |    Y    |    Y    |    Y    |    N    |
Write   |    Y    |    Y    |    Y    |    Y    |    N    |    N    |

- Should read as: if the system 'A' declared first has the row operation and system 'B' has the column operation
declared second, can they be executed in parallel to ensure a deterministic outcome and to respect declaration
order intuition?
- When 'Read/Write' operations are declared first, structural changes are not expected to be observed and as such,
they don't force an ordering.
- When 'Read/Write' operations are declared second, structural changes must be resolved for overlapping segments.
*1: Coherence is satisfied by those operations only if resolution is ordered. Note that batch operations such as
'add_all/remove_all/destroy_all' can easily overlap with other operations.
*2: Coherence is satisfied by those operations since the outcome is deterministic.
*3: Coherence is satisfied as long as components are not observed.

STATIC VS DYNAMIC:
- Static allows all segments to be known at initialization time, ensuring that scheduling is most efficient and knowns all dependencies.
- Dynamic makes entities hard to debug since a removed component loses all its data.

// PROS
// - Batch operations can be implemented efficiently if a whole segment is modified.

// CONS:
// - Adds many more segments and spreads entities more.
// - Adds many deferred operations that will be hard or inefficient to parallelize.
// - Adds a lot of complexity when trying to figure out where a component comes from.
// - Probably does not add much performance when taking into account the loss in parallelization and resolution factors.
// - Probably does not add much memory efficiency and is probably worse. While segments stores *might* be smaller overall,
// chances are that a lot of entity slots will be reserved more than once.
// - Make dependency management more complex.

TODO:
- Fix coherence when 'Create' and 'Destroy' appear in the same system or disallow those systems.
A 'Destroy::all' operation could destroy entities that have not been created yet since a later 'Create' might not need to
defer its operation. A possible solution would be for the 'Destroy::all' operation to store 'segment.reserved'.

- Similar to 'Create', move resolve logic of 'Emit' to run time when possible. As long as no resize are required,
it should be possible to do so by adding a 'reserved: AtomicUsize' to queues.

- Make deferral more explicit and extensible by enforcing the format: 'Defer<Add<Freeze>>'?

- Find a way to definitely know which segments overlap between deferred operations and 'Read/Write'. Otherwise,
the deferred operations should have a 'Defer(Entity)' dependency on all segments.
*/

// #[derive(Inject)]
// struct Boba<'a> {
//     pub time: &'a Time,
//     pub query: Query<'a, &'a Position>,
// }

#[derive(Message, Clone, Debug)]
enum Input {
    Left(bool),
    Right(bool),
    Down(bool),
    Up(bool),
}

#[derive(Component, Copy, Clone, Debug)]
struct Position(isize, isize);

#[derive(Component, Copy, Clone, Debug)]
struct Render {
    color: [f32; 4],
    visible: bool,
}

#[derive(Component, Copy, Clone, Debug)]
struct Controller;

#[derive(Resource, Default, Clone, Debug)]
struct Time {
    pub frames: usize,
    pub total: Duration,
    pub delta: Duration,
}

#[derive(Template)]
struct Player(Add<Position>, Add<Render>, Add<Controller>);

fn main() {
    run().unwrap();
}

fn run() -> Result<(), Box<dyn error::Error>> {
    const SIZE: [f64; 2] = [640., 480.];
    const CELLS: [f64; 2] = [25., 25.];

    let mut world = World::new();
    let mut window: PistonWindow = WindowSettings::new("Example_01", SIZE)
        .exit_on_esc(true)
        .build()?;

    // Initialize players.
    world.run(|mut create: Create<_>| {
        create.all((0..10).map(|i| {
            Player(
                Position(i, -i).into(),
                Render {
                    color: [0.1 * i as f32, 0.1, 0.8, 1.],
                    visible: true,
                }
                .into(),
                Controller.into(),
            )
        }));
    })?;
    //     |query: Query<Entity>, extract: Extract, create: Create<_>| {
    //         for entity in &query {
    //             let template/*: SomeClonableEntityRepresentation */ = extract.template(entity);
    //             create.one(template);
    //         }
    //     },

    let mut inputs = world.injector::<Emit<Input>>()?;
    let mut time = world.injector::<&mut Time>()?;
    let mut render = world.injector::<Query<(&Position, &Render)>>()?;
    let mut runner = world
        .scheduler()
        .pipe(print_fps)
        .add(apply_input_to_position)
        .schedule()?;

    while let Some(event) = window.next() {
        match event {
            Event::Input(
                piston::Input::Button(ButtonArgs {
                    state,
                    button: Button::Keyboard(key),
                    ..
                }),
                _,
            ) => inputs.run(&mut world, |mut inputs| match key {
                Key::Left => inputs.one(Input::Left(state == ButtonState::Press)),
                Key::Right => inputs.one(Input::Right(state == ButtonState::Press)),
                Key::Down => inputs.one(Input::Down(state == ButtonState::Press)),
                Key::Up => inputs.one(Input::Up(state == ButtonState::Press)),
                _ => {}
            })?,
            Event::Loop(Loop::Update(UpdateArgs { dt })) => {
                time.run(&mut world, |time| {
                    time.frames += 1;
                    time.delta = Duration::from_secs_f64(dt);
                    time.total += time.delta;
                })?;
                runner.run(&mut world)?
            }
            event => window
                .draw_2d(&event, |context, graphics, _| -> Result<_, Error> {
                    graphics.clear_color([0.25, 0.4, 0.1, 1.]);

                    let cell_size = [SIZE[0] / CELLS[0], SIZE[1] / CELLS[1]];
                    let square = rectangle::square(0., 0., cell_size[1]);
                    render.run(&mut world, |query| {
                        for (position, render) in &query {
                            if render.visible {
                                let x =
                                    position.0.rem_euclid(CELLS[0] as isize) as f64 * cell_size[0];
                                let y = (-position.1).rem_euclid(CELLS[1] as isize) as f64
                                    * cell_size[1];
                                let transform = context.transform.trans(x, y);
                                Rectangle::new(render.color).draw(
                                    square,
                                    &context.draw_state,
                                    transform,
                                    graphics,
                                );
                            }
                        }
                    })
                })
                .unwrap_or(Ok(()))?,
        }
    }

    Ok(())
}

fn apply_input_to_position(
    mut inputs: Receive<Input>,
    mut query: Query<&mut Position, Has<Controller>>,
) {
    for input in &mut inputs {
        match input {
            Input::Left(true) => query.each_mut(|position| position.0 -= 1),
            Input::Right(true) => query.each_mut(|position| position.0 += 1),
            Input::Down(true) => query.each_mut(|position| position.1 -= 1),
            Input::Up(true) => query.each_mut(|position| position.1 += 1),
            _ => {}
        }

        println!("{:?}", input);
    }
}

fn print_fps(scheduler: Scheduler) -> Scheduler {
    const SIZE: usize = 100;
    let mut history = VecDeque::new();
    scheduler.add(move |time: &Time| {
        history.push_back(time.delta);
        if history.len() < SIZE {
            return;
        }

        while history.len() > SIZE {
            history.pop_front();
        }

        let mut sum = Duration::from_secs(0);
        for &duration in history.iter() {
            sum += duration;
        }
        println!("{:?}", sum / SIZE as u32);
    })
}
