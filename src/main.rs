extern crate gl;
extern crate nalgebra as na;
extern crate sdl2;

#[macro_use]
pub mod gl_shaders;
mod circle;
pub mod gl_vertices;

use circle::*;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use std::time::Duration;

use rapier2d_f64::dynamics::{RigidBody, RigidBodyHandle};
use rapier2d_f64::geometry::{BroadPhase, ColliderBuilder, ColliderSet, NarrowPhase, SharedShape};
use rapier2d_f64::na::Vector2;
use rapier2d_f64::na::{ComplexField, Isometry2};
use rapier2d_f64::pipeline::PhysicsPipeline;
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
            .window("explain", 800, 600)
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
            gl::Viewport(0, 0, 800, 600);
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::DebugMessageCallback(Some(message_callback), std::ptr::null());
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
    }

    // initialize the physics
    let mut pipeline = PhysicsPipeline::new();
    let gravity = V2::new(0.0, 10.0);
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
        .build();
    let circle_collider = ColliderBuilder::new(SharedShape::ball(10.0))
        .restitution(0.8)
        .build();
    let circle_ref = bodies.insert(circle);
    let circle_collider_handle = colliders.insert(circle_collider, circle_ref, &mut bodies);

    let mut projection = nalgebra::Orthographic3::new(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);
    let mut camera = nalgebra::Matrix4::new_translation(&na::Vector3::new(0.0, 0.0, 0.0));
    let mut drawing_wireframe = false;

    'running: loop {
        // process
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

        // handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,

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

        // draw

        unsafe {
            gl::ClearColor(1.0, 1.0, 1.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            for (_handle, body) in bodies.iter() {
                if body.colliders().len() <= 0 {
                    continue;
                }
                match colliders
                    .get(body.colliders()[0])
                    .unwrap()
                    .shape()
                    .as_ball()
                {
                    Some(ball) => {
                        let mut drawn_circle = Circle::new();
                        drawn_circle.offset = na::convert(body.position().translation.vector);
                        drawn_circle.radius = ball.radius as f32;
                        drawn_circle.draw(projection.as_matrix(), &camera);
                    }
                    None => (),
                }
            }
        }
        window.gl_swap_window();

        // idle
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60)); // TODO take exactly 1/60s every time by accounting for how long computation above takes
    }
}
