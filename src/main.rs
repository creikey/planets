extern crate gl;
extern crate nalgebra as na;
extern crate sdl2;

#[macro_use]
pub mod gl_shaders;
pub mod gl_vertices;
mod quick_draw;

use quick_draw::*;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use std::time::Duration;

use rapier2d_f64::dynamics::{RigidBody, RigidBodyHandle};
use rapier2d_f64::geometry::{
    BroadPhase, ColliderBuilder, ColliderSet, NarrowPhase, Ray, SharedShape,
};
use rapier2d_f64::na::Vector2;
use rapier2d_f64::na::{ComplexField, Isometry2};
use rapier2d_f64::pipeline::{PhysicsPipeline, QueryPipeline};
use rapier2d_f64::{
    dynamics::{CCDSolver, IntegrationParameters, JointSet, RigidBodyBuilder, RigidBodySet},
    na::Translation2,
};

// TODO put these type aliases into util mod
type P2 = na::Point2<f64>;
type V2 = na::Vector2<f64>;

// https://www.khronos.org/opengl/wiki/OpenGL_Error
extern "system" fn message_callback(
    source: gl::types::GLenum,
    t: gl::types::GLenum,
    id: gl::types::GLuint,
    severity: gl::types::GLenum,
    length: gl::types::GLsizei,
    message: *const gl::types::GLchar,
    user_param: *mut gl::types::GLvoid,
) {
    unsafe {
        let is_error = t == gl::DEBUG_TYPE_ERROR;

        let type_name = if is_error {
            String::from("ERROR")
        } else {
            format!("Type {}", t)
        };
        if is_error {
            println!(
                "GL {}: {}",
                type_name,
                std::ffi::CStr::from_ptr(message).to_str().unwrap()
            );
        }
    }
}

fn main() {
    // initialize sdl2 and opengl
    let sdl_context;
    let mut event_pump;
    let window;
    let _ctx; // when this is dropped the opengl context is destroyed
    {
        sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        event_pump = sdl_context.event_pump().unwrap();

        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::GLES);
        gl_attr.set_context_major_version(2);
        gl_attr.set_context_minor_version(0);

        window = video_subsystem
            .window("explain", 1000, 900)
            .opengl()
            .position_centered()
            .resizable()
            .build()
            .unwrap();

        _ctx = window.gl_create_context().unwrap();
        gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);

        debug_assert_eq!(gl_attr.context_profile(), GLProfile::GLES);
        debug_assert_eq!(gl_attr.context_version(), (2, 0));

        unsafe {
            gl::Viewport(0, 0, 1000, 900);
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::DebugMessageCallback(Some(message_callback), std::ptr::null());
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
    }

    // initialize the physics
    let mut pipeline = PhysicsPipeline::new();
    let mut query = QueryPipeline::new();
    let gravity = V2::new(0.0, 50.0);
    let integration_parameters = IntegrationParameters::default();
    let mut broad_phase = BroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let mut joints = JointSet::new();
    let mut ccd_solver = CCDSolver::new();
    // We ignore physics hooks and contact events for now.
    let physics_hooks = ();
    let event_handler = ();

    let circle = RigidBodyBuilder::new_dynamic()
        .position(Isometry2::new(V2::new(0.0, 0.0), 0.0))
        .linear_damping(0.5)
        .build();
    let circle_collider = ColliderBuilder::new(SharedShape::ball(1.0))
        .restitution(0.0)
        .build();
    let circle_ref = bodies.insert(circle);
    let circle_collider_handle = colliders.insert(circle_collider, circle_ref, &mut bodies);

    fn new_static_box(pos: V2, hx: f64, hy: f64, colliders: &mut ColliderSet, bodies: &mut RigidBodySet) {
        let floor = RigidBodyBuilder::new_static()
            .position(Isometry2::new(pos, 0.0f64.to_radians()))
            .build();
        let floor_collider = ColliderBuilder::new(SharedShape::cuboid(hx, hy))
            .restitution(0.2)
            .build();
        let floor_ref = bodies.insert(floor);
        let floor_collider_handle = colliders.insert(floor_collider, floor_ref, bodies);
    }
    new_static_box(V2::new(0.0, 100.0), 800.0, 10.0, &mut colliders, &mut bodies);
    new_static_box(V2::new(-50.0, 100.0), 10.0, 100.0, &mut colliders, &mut bodies);
    new_static_box(V2::new(75.0, 100.0), 10.0, 100.0, &mut colliders, &mut bodies);

    let mut projection = nalgebra::Orthographic3::new(0.0, 1000.0, 900.0, 0.0, -1.0, 1.0);
    let mut camera = nalgebra::Matrix4::new_translation(&na::Vector3::new(400.0, 0.0, 0.0));
    camera *= na::Matrix4::new_scaling(8.0);
    let mut drawing_wireframe = false;

    // TODO figure out a way to duplicate the keyboard state for "is_just_pressed" functionality
    let mut jump_pressed_last_frame = false;
    let mut jump_pressed = false;

    'running: loop {
        // handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,

                // player jumping
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    ..
                } => jump_pressed = true,
                Event::KeyUp {
                    keycode: Some(Keycode::W),
                    ..
                } => jump_pressed = false,

                // debug wireframe mode
                #[cfg(debug_assertions)]
                Event::KeyDown {
                    keycode: Some(Keycode::Z),
                    ..
                } => {
                    drawing_wireframe = !drawing_wireframe;
                    unsafe {
                        if drawing_wireframe {
                            gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
                        } else {
                            gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
                        }
                    }
                }

                // resize the gl canvas with the window
                Event::Window { win_event, .. } => match win_event {
                    sdl2::event::WindowEvent::Resized(x, y) => unsafe {
                        gl::Viewport(0, 0, x, y);
                        projection.set_right(x as f32);
                        projection.set_bottom(y as f32);
                    },
                    _ => {}
                },
                _ => {}
            }
        }

        // physics process
        pipeline.step(
            &gravity,
            &integration_parameters,
            &mut broad_phase,
            &mut narrow_phase,
            &mut bodies,
            &mut colliders,
            &mut joints,
            &mut ccd_solver,
            &physics_hooks,
            &event_handler,
        );
        query.update(&bodies, &colliders);
        {
            let horizontal_movement = event_pump
                .keyboard_state()
                .is_scancode_pressed(sdl2::keyboard::Scancode::D)
                as i32 as f64
                - event_pump
                    .keyboard_state()
                    .is_scancode_pressed(sdl2::keyboard::Scancode::A) as i32
                    as f64;
            let circle_body = bodies.get_mut(circle_ref).unwrap();
            circle_body.apply_force(V2::new(500.0 * horizontal_movement, 0.0), true);
            if !jump_pressed_last_frame
                && jump_pressed
                && query
                    .cast_ray(
                        &colliders,
                        &rapier2d_f64::geometry::Ray::new(
                            na::Point2::from(circle_body.position().translation.vector),
                            V2::new(0.0, 1.0),
                        ),
                        1.5,
                        true,
                        rapier2d_f64::geometry::InteractionGroups::all(),
                        Some(&|ch, c| ch != circle_collider_handle),
                    )
                    .is_some()
            {
                circle_body.apply_impulse(V2::new(0.0, -200.0), true);
            }
        }

        // draw

        unsafe {
            gl::ClearColor(1.0, 1.0, 1.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            let qd = DrawingContext {
                projection: &projection.as_matrix(),
                camera: &camera,
            };
            for (_handle, body) in bodies.iter() {
                if body.colliders().len() <= 0 {
                    continue;
                }
                match colliders
                    .get(body.colliders()[0])
                    .unwrap()
                    .shape()
                    .as_typed_shape()
                {
                    rapier2d_f64::geometry::TypedShape::Ball(ball) => {
                        qd.draw_circle(
                            na::convert(body.position().translation.vector),
                            ball.radius as f32,
                        );
                    }
                    rapier2d_f64::geometry::TypedShape::Cuboid(cube) => {
                        qd.draw_rect_rot(
                            na::convert(body.position().translation.vector - cube.half_extents),
                            na::convert(body.position().translation.vector + cube.half_extents),
                            body.position().rotation.angle() as f32,
                        );
                    }
                    _ => (),
                }
            }
        }
        window.gl_swap_window();

        jump_pressed_last_frame = event_pump
            .keyboard_state()
            .is_scancode_pressed(sdl2::keyboard::Scancode::W);
        // idle
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60)); // TODO take exactly 1/60s every time by accounting for how long computation above takes
    }
}
