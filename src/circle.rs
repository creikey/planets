extern crate gl;
use crate::gl_shaders::*;
use crate::gl_vertices::*;

type P2 = na::Point2<f32>;
type V2 = na::Vector2<f32>;

pub struct Circle {
    shader_program: ShaderProgram,
    gl_vertices: VertexData<V2>,
    pub offset: V2,
    pub radius: f32,
}

impl Circle {
    pub fn new() -> Self {
        let shader_program = shader!("circle.vert", "circle.frag");

        use vertex_attribs::*;
        let mut gl_vertices = VertexData::new(vec![VECTOR2_F32]);

        gl_vertices.append(
            &mut vec![
                V2::new(-1.0, -1.0),
                V2::new(1.0, -1.0),
                V2::new(1.0, 1.0),
                V2::new(-1.0, 1.0),
            ],
            &mut vec![0, 1, 2, 0, 3, 2],
            true,
        );

        Circle {
            shader_program,
            gl_vertices,
            offset: V2::new(0.0, 0.0),
            radius: 0.0,
        }
    }

    pub fn draw(&self, projection: &na::Matrix4<f32>, camera: &na::Matrix4<f32>) {
        self.shader_program.set_used();
        self.shader_program.write_mat4("projection", projection);
        self.shader_program.write_mat4("camera", camera);
        self.shader_program.write_vec2("offset", &self.offset);
        self.shader_program.write_float("radius", self.radius);
        self.gl_vertices.draw();
    }
}
