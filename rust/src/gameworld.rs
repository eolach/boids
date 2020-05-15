use gdextras::input::InputEventExt;
use gdextras::node_ext::NodeExt;
use gdnative::{
    godot_error, godot_wrap_method, godot_wrap_method_inner, godot_wrap_method_parameter_count,
    methods, InputEvent, NativeClass, Node2D, Rect2, Vector2, InputEventMouse, Sprite
};
use legion::prelude::*;
use rand::prelude::*;

use crate::boids::{Acceleration, Boid, Velocity, Pos, Forces, add_boid_systems};
use crate::spawner;
const BOID_COUNT: usize = 80;

fn physics_systems() -> Schedule {
    let schedule = Schedule::builder();
    let schedule = add_boid_systems(schedule);
    schedule.build()
}

// -----------------------------------------------------------------------------
//     - Resources -
// -----------------------------------------------------------------------------
pub struct Delta(pub f32);
pub struct CohesionMul(pub f32);
pub struct SeparationMul(pub f32);
pub struct AlignmentMul(pub f32);
pub struct ShouldFlee(pub bool);
pub struct ShouldSeek(pub bool);

pub struct Target(pub Sprite);

unsafe impl Send for Target {}
unsafe impl Sync for Target {}

#[derive(Debug, Clone, Copy)]
pub struct Viewport(pub Rect2);

impl Viewport {
    fn from_vec2(size: Vector2) -> Self {
        let origin = size / 2.;
        let rect = Rect2::new(-origin.to_point(), size.to_size());
        Self(rect)
    }
}

// -----------------------------------------------------------------------------
//     - Godot node -
// -----------------------------------------------------------------------------

#[derive(NativeClass)]
#[inherit(Node2D)]
pub struct GameWorld {
    world: World,
    physics: Schedule,
    resources: Resources,
}

#[methods]
impl GameWorld {
    pub fn _init(_owner: Node2D) -> Self {
        let mut resources = Resources::default();

        // Resources
        resources.insert(Delta(0.));
        resources.insert(CohesionMul(1.0));
        resources.insert(SeparationMul(1.0));
        resources.insert(AlignmentMul(1.0));
        resources.insert(ShouldSeek(false));
        resources.insert(ShouldFlee(false));

        let physics = physics_systems();

        Self {
            world: Universe::new().create_world(),
            resources,
            physics,
        }
    }

    #[export]
    pub unsafe fn _ready(&mut self, mut owner: Node2D) {
        let mut rng = thread_rng();

        // Add target
        let target = owner.get_and_cast::<Sprite>("Target").expect("failed to get the target");
        self.resources.insert(Target(target));

        // Add viewport rect
        let size = owner.get_viewport().unwrap().get_size();
        let viewport = Viewport::from_vec2(size);
        self.resources.insert(viewport);

        for _ in 0..BOID_COUNT {
            let mut boid = spawner::spawn_boid();
            let x = rng.gen_range(viewport.0.min_x(), viewport.0.max_x());
            let y = rng.gen_range(viewport.0.min_y(), viewport.0.max_y());

            let pos = Vector2::new(x, y);
            owner.add_child(Some(boid.to_node()), false);
            boid.set_global_position(pos);

            let velocity = Vector2::new(rng.gen_range(-500., 500.), rng.gen_range(-500., 500.))
                .normalize()
                * 500f32;

            self.world.insert(
                (),
                Some((
                    Boid(boid),
                    Velocity(velocity),
                    Acceleration(Vector2::zero()),
                    Pos(pos),
                    Forces::zero(),
                )),
            );
        }
    }

    #[export]
    pub fn _unhandled_input(&self, owner: Node2D, event: InputEvent) {
        if event.action_pressed("ui_cancel") {
            unsafe { owner.get_tree().map(|mut tree| tree.quit(0)) };
        }

        if let Some(ev) = event.cast::<InputEventMouse>() {
            if ev.is_pressed() {
                unsafe {
                    let pos = owner.get_global_mouse_position();
                    self.resources.get_mut::<Target>().map(|mut target| target.0.set_global_position(pos));
                }
            }
        }
    }

    // -----------------------------------------------------------------------------
    //     - signals -
    // -----------------------------------------------------------------------------
    #[export]
    pub fn _physics_process(&mut self, owner: Node2D, delta: f64) {
        self.resources
            .get_mut::<Delta>()
            .map(|mut d| d.0 = delta as f32);
        self.physics.execute(&mut self.world, &mut self.resources);
    }

    #[export]
    pub fn cohesion_value_changed(&mut self, owner: Node2D, val: f32) {
        self.resources.get_mut::<CohesionMul>().map(|mut mul| mul.0 = val);
    }

    #[export]
    pub fn separation_value_changed(&mut self, owner: Node2D, val: f32) {
        self.resources.get_mut::<SeparationMul>().map(|mut mul| mul.0 = val);
    }

    #[export]
    pub fn alignment_value_changed(&mut self, owner: Node2D, val: f32) {
        self.resources.get_mut::<AlignmentMul>().map(|mut mul| mul.0 = val);
    }

    #[export]
    pub fn seek_toggled(&mut self, owner: Node2D, toggle: bool) {
        self.resources.get_mut::<ShouldSeek>().map(|mut seek| seek.0 = toggle);
    }

    #[export]
    pub fn flee_toggled(&mut self, owner: Node2D, toggle: bool) {
        self.resources.get_mut::<ShouldFlee>().map(|mut flee| flee.0 = toggle);
        eprintln!("{:?}", "flee toggle");
    }
}
