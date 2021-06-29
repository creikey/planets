extern crate gl;
use crate::gl_shaders::*;
use crate::gl_vertices::*;

type P2 = na::Point2<f32>;
type V2 = na::Vector2<f32>;

pub struct DrawingContext<'a> {
    pub projection: &'a na::Matrix4<f32>,
    pub camera: &'a na::Matrix4<f32>,
}

impl<'a> DrawingContext<'a> {
    pub fn draw_circle(&self, offset: V2, radius: f32) {
        let shader_program = shader_inline!(
            "#version 330 core

            layout (location = 0) in vec2 Position;
            
            out vec2 pos;
            
            uniform mat4 camera;
            uniform mat4 projection;
            uniform vec2 offset;
            uniform float radius;
            
            void main()
            {
                pos = Position;
                gl_Position = projection * camera * vec4(Position * radius + offset, 0.0, 1.0);
            }
            ",
            "#version 330 core

            out vec4 Color;
            in vec2 pos;
            
            void main()
            {
                float len = length(pos);
                if(len > 1.0) {
                    Color = vec4(0.0);
                } else {
                    Color = vec4(0.0, 0.0, 0.0, 1.0);
                }
            }
            "
        );

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

        shader_program.set_used();
        shader_program.write_mat4("projection", self.projection);
        shader_program.write_mat4("camera", self.camera);
        shader_program.write_vec2("offset", &offset);
        shader_program.write_float("radius", radius);
        gl_vertices.draw();
    }
    fn rot_mat(t: f32, wrt: V2) -> na::Matrix3<f32> {
        na::Matrix3::new(
            t.cos(),
            -t.sin(),
            -wrt.x * t.cos() + wrt.y * t.sin() + wrt.x,
            t.sin(),
            t.cos(),
            -wrt.x * t.sin() - wrt.y * t.cos() + wrt.y,
            0.0,
            0.0,
            1.0,
        )
    }
    pub fn draw_rect(&self, upper_left: V2, lower_right: V2) {
        self.draw_rect_rot(upper_left, lower_right, 0.0);
    }
    /// `rotation` is with respect to the center of the rectangle
    pub fn draw_rect_rot(&self, upper_left: V2, lower_right: V2, rotation: f32) {
        let shader_program = shader_inline!(
            "#version 330 core

            layout (location = 0) in vec2 Position;
            
            out vec2 pos;
            
            uniform mat4 camera;
            uniform mat4 projection;
            
            void main()
            {
                pos = Position;
                gl_Position = projection * camera * vec4(Position, 0.0, 1.0);
            }
            ",
            "#version 330 core

            out vec4 Color;
            in vec2 pos;
            
            void main()
            {
                Color = vec4(0.0, 0.0, 0.0, 1.0);
            }
            "
        );

        use vertex_attribs::*;
        let mut gl_vertices = VertexData::new(vec![VECTOR2_F32]);

        let width = lower_right.x - upper_left.x;
        let rotation = Self::rot_mat(rotation, (lower_right + upper_left)/2.0);
        gl_vertices.append(
            &mut vec![
                rotation.transform_point(&P2::from(upper_left)).coords,
                rotation.transform_point(&P2::from(upper_left + V2::new(width, 0.0))).coords,
                rotation.transform_point(&P2::from(lower_right)).coords,
                rotation.transform_point(&P2::from((lower_right - V2::new(width, 0.0)))).coords,
            ],
            &mut vec![0, 1, 2, 0, 3, 2],
            true,
        );

        shader_program.set_used();
        shader_program.write_mat4("projection", self.projection);
        shader_program.write_mat4("camera", self.camera);
        gl_vertices.draw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rot_mat_works() {
        let rot = DrawingContext::rot_mat(-1.5708, V2::new(1.0, 0.0));
        assert_eq!(rot.transform_point(&P2::new(1.0, 1.0)), P2::new(2.0, 0.0));
    }
}