use gdnative::{Sprite, Vector2};
use legion::prelude::*;
use legion::systems::schedule::Builder;

use crate::gameworld::{Delta, Viewport, CohesionMul, SeparationMul, AlignmentMul};

// -----------------------------------------------------------------------------
//     - Components -
// -----------------------------------------------------------------------------
pub struct Boid(pub Sprite);

unsafe impl Send for Boid {}
unsafe impl Sync for Boid {}

pub struct Velocity(pub Vector2);
pub struct Acceleration(pub Vector2);
pub struct Pos(pub Vector2);

pub struct Forces {
    cohesion: Vector2,
    separation: Vector2,
    alignment: Vector2,
}

impl Forces {
    pub fn zero() -> Self {
        Self {
            cohesion: Vector2::zero(), 
            separation: Vector2::zero(), 
            alignment: Vector2::zero(), 
        }
    }

    fn reset(&mut self) {
        *self = Self::zero();
    }
}

const MAX_SPEED: f32 = 500.;

// -----------------------------------------------------------------------------
//     - Systems -
// -----------------------------------------------------------------------------

fn cohesion() -> Box<dyn Runnable> {
    SystemBuilder::new("cohesion")
        .with_query(<(Read<Pos>, Write<Forces>)>::query())
        .build_thread_local(|_, world, _, query| {
            let all_positions = query.iter_mut(world).map(|(pos, _)| pos.0).collect::<Vec<_>>();
            let neighbour_distance = 200f32;

            for (pos, mut force) in query.iter_mut(world) {
                let mut count = 0;

                for other_pos in &all_positions {
                    let distance = (*other_pos - pos.0).length();

                    if distance < neighbour_distance {
                        count += 1;
                        force.cohesion += *other_pos;
                    }
                }

                if count > 0 {
                    force.cohesion /= count as f32;
                    force.cohesion -= pos.0;
                }
            }
        })
}

fn separation() -> Box<dyn Runnable> {
    SystemBuilder::new("separation")
        .with_query(<(Read<Pos>, Write<Forces>)>::query())
        .build_thread_local(|cmd, world, resources, query| {
            let all_positions = query.iter_mut(world).map(|(pos, _)| pos.0).collect::<Vec<_>>();
            let neighbour_distance = 100f32;

            for (pos, mut force) in query.iter_mut(world) {
                let mut count = 0;

                for other_pos in &all_positions {
                    let distance = (*other_pos - pos.0).length();

                    if distance < neighbour_distance {
                        count += 1;
                        force.separation += pos.0 - *other_pos;
                    }
                }

                if count > 0 {
                    force.separation /= count as f32;
                }
            }
        })
}

fn alignment() -> Box<dyn Runnable> {
    SystemBuilder::new("alignment")
        .with_query(<(Read<Pos>, Read<Velocity>, Write<Forces>)>::query())
        .build_thread_local(|cmd, world, resources, query| {
            let all_positions = query.iter_mut(world).map(|(pos, vel, _)| (pos.0, vel.0)).collect::<Vec<_>>();
            let neighbour_distance = 100f32;
            
            for (pos, vel, mut force) in query.iter_mut(world) {
                let mut count = 0;

                for (other_pos, other_vel) in &all_positions {
                    let distance = (*other_pos - pos.0).length();

                    if distance < neighbour_distance {
                        count += 1;
                        force.alignment += *other_vel;
                    }
                }

                if count > 0 {
                    force.alignment /= count as f32;
                }
            }
        })
}

fn reset_acceleration() -> Box<dyn Runnable> {
    SystemBuilder::new("reset acceleration")
        .with_query(<Write<Acceleration>>::query())
        .build_thread_local(|_, world, _, accelerations| {
            for mut acc in accelerations.iter_mut(world) {
                acc.0 = Vector2::zero();
            }
        })
}

fn reset_forces() -> Box<dyn Runnable> {
    SystemBuilder::new("reset forces")
        .with_query(<Write<Forces>>::query())
        .build_thread_local(|_, world, _, accelerations| {
            for mut force in accelerations.iter_mut(world) {
                force.reset();
            }
        })
}

fn screen_wrap() -> Box<dyn Runnable> {
    SystemBuilder::new("sceen_wrap")
        .read_resource::<Viewport>()
        .with_query(<(Write<Pos>, Write<Boid>)>::query())
        .build_thread_local(|_, world, viewport, boids| unsafe {
            let offset = 16.;
            for (mut pos, mut boid) in boids.iter_mut(world) {
                if pos.0.x < viewport.0.min_x() - offset {
                    pos.0.x = viewport.0.max_x() + offset;
                    boid.0.set_global_position(pos.0);
                } else if pos.0.x > viewport.0.max_x() + offset {
                    pos.0.x = viewport.0.min_x() - offset;
                    boid.0.set_global_position(pos.0);
                }

                if pos.0.y < viewport.0.min_y() - offset {
                    pos.0.y = viewport.0.max_y() + offset;
                    boid.0.set_global_position(pos.0);
                } else if pos.0.y > viewport.0.max_y() + offset {
                    pos.0.y = viewport.0.min_y() - offset;
                    boid.0.set_global_position(pos.0);
                }
            }
        })
}

fn move_boids() -> Box<dyn Runnable> {
    SystemBuilder::new("move_boids")
        .read_resource::<Delta>()
        .with_query(<(
            Read<Acceleration>,
            Write<Velocity>,
            Write<Pos>,
            Write<Boid>,
        )>::query())
        .build_thread_local(|_, world, delta, query| unsafe {
            for (acc, mut vel, mut pos, mut boid) in query.iter_mut(world) {
                vel.0 += acc.0;
                vel.0 = vel.0.with_max_length(MAX_SPEED);
                boid.0.global_translate(vel.0 * delta.0);
                pos.0 = boid.0.get_global_position();
            }
        })
}

fn rotate() -> Box<dyn Runnable> {
    SystemBuilder::new("rotate")
        .with_query(<(Write<Boid>, Read<Velocity>)>::query())
        .build_thread_local(|_, world, _, query| {
            for (mut boid, vel) in query.iter_mut(world) {
                let rot = vel.0.y.atan2(vel.0.x) as f64;
                unsafe { boid.0.set_global_rotation(rot) };
            }
        })
}

fn apply_forces() -> Box<dyn Runnable> {
    SystemBuilder::new("apply forces")
        .read_resource::<CohesionMul>()
        .read_resource::<SeparationMul>()
        .read_resource::<AlignmentMul>()
        .with_query(<(Read<Forces>, Write<Acceleration>)>::query())
        .build_thread_local(|cmd, world, resources, query| {
            let (cohesion_mul, separation_mul, alignment_mul) = resources;
            for (force, mut acc) in query.iter_mut(world) {
                acc.0 += force.cohesion * cohesion_mul.0;
                acc.0 += force.separation * separation_mul.0;
                acc.0 += force.alignment * alignment_mul.0;
            }
        })
}

pub fn add_boid_systems(builder: Builder) -> Builder {
    builder
        .add_thread_local(reset_acceleration())
        .add_thread_local(reset_forces())
        .add_thread_local(cohesion())
        .add_thread_local(separation())
        .add_thread_local(alignment())
        .add_thread_local(apply_forces())
        .add_thread_local(move_boids())
        .add_thread_local(rotate())
        .add_thread_local(screen_wrap())
}
