use glium::{implement_vertex, uniform, Surface};

#[derive(Clone, Copy)]
pub struct BondSprite {
    pub end1: [f32; 2],
    pub end2: [f32; 2],
    pub radius1: f32,
    pub radius2: f32,
    pub color: [f32; 3],
}

implement_vertex!(BondSprite, end1, end2, radius1, radius2, color);

pub struct BondDrawing {
    pub shader_program: glium::Program,
    pub indices: glium::index::NoIndices,
}

impl BondDrawing {
    pub fn new(display: &glium::Display) -> Self {
        BondDrawing {
            shader_program: glium::Program::from_source(
                display,
                Self::VERTEX_SHADER_SRC,
                Self::FRAGMENT_SHADER_SRC,
                Some(Self::GEOMETRY_SHADER_SRC),
            )
            .unwrap(),
            indices: glium::index::NoIndices(glium::index::PrimitiveType::Points),
        }
    }

    pub fn draw<T>(
        &self,
        frame: &mut glium::Frame,
        vertex_buffer: &glium::VertexBuffer<T>,
        screen_transform: [[f32; 4]; 4],
    ) where
        T: Copy,
    {
        let uniforms = uniform! {
            screen_transform: screen_transform,
        };
        frame
            .draw(
                vertex_buffer,
                &self.indices,
                &self.shader_program,
                &uniforms,
                &Default::default(),
            )
            .unwrap();
    }

    const VERTEX_SHADER_SRC: &'static str = r#"
        #version 330 core

        in vec2 end1;
        in vec2 end2;
        in float radius1;
        in float radius2;
        in vec3 color;

        out BondSprite {
            vec2 end1;
            vec2 end2;
            float radius1;
            float radius2;
            vec3 color;
        } bond_out;

        void main() {
            bond_out.end1 = end1;
            bond_out.end2 = end2;
            bond_out.radius1 = radius1;
            bond_out.radius2 = radius2;
            bond_out.color = color;
        }
    "#;

    const GEOMETRY_SHADER_SRC: &'static str = r#"
        #version 330 core

        uniform mat4 screen_transform;

        layout (points) in;
        layout (triangle_strip, max_vertices = 4) out;

        in BondSprite {
            vec2 end1;
            vec2 end2;
            float radius1;
            float radius2;
            vec3 color;
        } bond_in[];

        out BondPoint {
            vec3 color;
        } bond_point_out;

        void emit_corner(in vec2 corner, in vec3 color) {
            bond_point_out.color = color;
            gl_Position = screen_transform * vec4(corner[0], corner[1], 0.0, 1.0);
            EmitVertex();
        }

        void emit_quad(in vec2 end1, in vec2 end2, in float radius1, in float radius2, in vec3 color) {
            emit_corner(corners, 0, 1, color);
            emit_corner(corners, 0, 3, color);
            emit_corner(corners, 2, 1, color);
            emit_corner(corners, 2, 3, color);
            EndPrimitive();
        }

        void main() {
            emit_quad(bond_in[0].end1, bond_in[0].end2, bond_in[0].radius1, bond_in[0].radius2, bond_in[0].color);
        }
    "#;

    const FRAGMENT_SHADER_SRC: &'static str = r#"
        #version 330 core

        in BondPoint {
            vec3 color;
        } bond_point_in;

        out vec4 color_out;

        void main() {
            color_out = vec4(bond_point_in.color, 1.0);
        }
    "#;
}
