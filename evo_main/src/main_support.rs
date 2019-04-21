use evo_model::world::World;
use crate::view::*;

pub fn init_and_run(world: World) {
    let view = View::new(world.min_corner(), world.max_corner());
    run(world, view);
}

fn run(mut world: World, mut view: View) {
    let mut done = false;
    while !done {
        world.tick();
        done = !view.render(&world);
    }
}
