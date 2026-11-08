//! Vector and matrix helpers used by the software render pipeline.
//!
//! Vectors come from `nalgebra-glm`, as in the course material.  The small
//! `Mat4` type is kept explicit so the model -> view -> projection steps can
//! be inspected and modified without hiding the math behind a GPU API.

use nalgebra_glm as glm;

pub type Vec3 = glm::Vec3;

pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
    glm::vec3(x, y, z)
}

pub fn dot(a: Vec3, b: Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    vec3(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

pub fn length(v: Vec3) -> f32 {
    dot(v, v).sqrt()
}

pub fn normalize(v: Vec3) -> Vec3 {
    let magnitude = length(v);
    if magnitude <= 1.0e-6 {
        vec3(0.0, 0.0, 0.0)
    } else {
        v / magnitude
    }
}

pub fn distance(a: Vec3, b: Vec3) -> f32 {
    length(a - b)
}

pub fn clamp(value: f32, min_value: f32, max_value: f32) -> f32 {
    value.max(min_value).min(max_value)
}

pub fn lerp_vec3(a: Vec3, b: Vec3, amount: f32) -> Vec3 {
    a + (b - a) * amount
}

pub fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = clamp((value - edge0) / (edge1 - edge0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Clone, Copy, Debug)]
pub struct Mat4 {
    /// Row-major matrix.  Points are treated as column vectors.
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    pub fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn translation(position: Vec3) -> Self {
        let mut result = Self::identity();
        result.m[0][3] = position.x;
        result.m[1][3] = position.y;
        result.m[2][3] = position.z;
        result
    }

    pub fn scale(scale: Vec3) -> Self {
        let mut result = Self::identity();
        result.m[0][0] = scale.x;
        result.m[1][1] = scale.y;
        result.m[2][2] = scale.z;
        result
    }

    pub fn rotation_x(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, c, -s, 0.0],
                [0.0, s, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_y(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [
                [c, 0.0, s, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [-s, 0.0, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_z(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            m: [
                [c, -s, 0.0, 0.0],
                [s, c, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Camera looking along its local +Z axis.  This convention makes the
    /// software projection and depth test easy to follow.
    pub fn look_at(eye: Vec3, target: Vec3, world_up: Vec3) -> Self {
        let forward = normalize(target - eye);
        let right = normalize(cross(forward, world_up));
        let up = normalize(cross(right, forward));

        Self {
            m: [
                [right.x, right.y, right.z, -dot(right, eye)],
                [up.x, up.y, up.z, -dot(up, eye)],
                [forward.x, forward.y, forward.z, -dot(forward, eye)],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Perspective projection for a camera whose visible direction is +Z.
    /// The resulting normalized depth is in approximately [0, 1].
    pub fn perspective(fov_y_radians: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y_radians * 0.5).tan();
        let range = far / (far - near);
        Self {
            m: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, range, -near * range],
                [0.0, 0.0, 1.0, 0.0],
            ],
        }
    }

    pub fn transform_vec4(self, v: [f32; 4]) -> [f32; 4] {
        let mut result = [0.0; 4];
        for (row, output) in self.m.iter().zip(result.iter_mut()) {
            *output = row[0] * v[0]
                + row[1] * v[1]
                + row[2] * v[2]
                + row[3] * v[3];
        }
        result
    }

    pub fn transform_point(self, point: Vec3) -> Vec3 {
        let transformed = self.transform_vec4([point.x, point.y, point.z, 1.0]);
        if transformed[3].abs() > 1.0e-6 {
            vec3(
                transformed[0] / transformed[3],
                transformed[1] / transformed[3],
                transformed[2] / transformed[3],
            )
        } else {
            vec3(transformed[0], transformed[1], transformed[2])
        }
    }

    pub fn transform_direction(self, direction: Vec3) -> Vec3 {
        let transformed = self.transform_vec4([direction.x, direction.y, direction.z, 0.0]);
        normalize(vec3(transformed[0], transformed[1], transformed[2]))
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Mat4;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = [[0.0; 4]; 4];
        for (row_index, row) in result.iter_mut().enumerate() {
            for (column_index, value) in row.iter_mut().enumerate() {
                for k in 0..4 {
                    *value += self.m[row_index][k] * rhs.m[k][column_index];
                }
            }
        }
        Mat4 { m: result }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_moves_a_point() {
        let transform = Mat4::translation(vec3(2.0, 3.0, 4.0));
        let result = transform.transform_point(vec3(1.0, 1.0, 1.0));
        assert!((result.x - 3.0).abs() < 1.0e-5);
        assert!((result.y - 4.0).abs() < 1.0e-5);
        assert!((result.z - 5.0).abs() < 1.0e-5);
    }

    #[test]
    fn camera_sees_points_in_front() {
        let view = Mat4::look_at(
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, -1.0),
            vec3(0.0, 1.0, 0.0),
        );
        let point = view.transform_point(vec3(0.0, 0.0, -10.0));
        assert!((point.z - 10.0).abs() < 1.0e-5);
    }
}
