//! Mesh representation and a deliberately small Wavefront OBJ loader.
//!
//! The importer handles positions (`v`), texture coordinates (`vt`), normals
//! (`vn`) and polygon faces (`f`). Polygons are triangulated with a fan.

use crate::math::{cross, normalize, vec3, Vec3};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<[usize; 3]>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            triangles: Vec::new(),
        }
    }

    pub fn uv_sphere(latitude_steps: usize, longitude_steps: usize) -> Self {
        let mut mesh = Self::new();
        let rows = latitude_steps.max(3);
        let columns = longitude_steps.max(3);

        for row in 0..=rows {
            let v = row as f32 / rows as f32;
            let latitude = std::f32::consts::PI * (v - 0.5);
            let y = latitude.sin();
            let ring = latitude.cos();
            for column in 0..=columns {
                let u = column as f32 / columns as f32;
                let longitude = u * std::f32::consts::TAU;
                let position = vec3(ring * longitude.cos(), y, ring * longitude.sin());
                mesh.vertices.push(Vertex {
                    position,
                    normal: normalize(position),
                    uv: [u, 1.0 - v],
                });
            }
        }

        for row in 0..rows {
            for column in 0..columns {
                let a = row * (columns + 1) + column;
                let b = a + columns + 1;
                mesh.triangles.push([a, b, a + 1]);
                mesh.triangles.push([a + 1, b, b + 1]);
            }
        }
        mesh
    }

    pub fn plane_xz(width: f32, depth: f32, divisions: usize, y: f32) -> Self {
        let mut mesh = Self::new();
        let steps = divisions.max(1);
        for row in 0..=steps {
            for column in 0..=steps {
                let u = column as f32 / steps as f32;
                let v = row as f32 / steps as f32;
                let x = (u - 0.5) * width;
                let z = (v - 0.5) * depth;
                mesh.vertices.push(Vertex {
                    position: vec3(x, y, z),
                    normal: vec3(0.0, 1.0, 0.0),
                    uv: [u, v],
                });
            }
        }
        for row in 0..steps {
            for column in 0..steps {
                let a = row * (steps + 1) + column;
                let b = a + steps + 1;
                // Winding faces upward.
                mesh.triangles.push([a, b, a + 1]);
                mesh.triangles.push([a + 1, b, b + 1]);
            }
        }
        mesh
    }

    pub fn ring(inner_radius: f32, outer_radius: f32, segments: usize) -> Self {
        let mut mesh = Self::new();
        let count = segments.max(12);
        for index in 0..=count {
            let angle = index as f32 / count as f32 * std::f32::consts::TAU;
            let (s, c) = angle.sin_cos();
            mesh.vertices.push(Vertex {
                position: vec3(c * inner_radius, s * inner_radius, 0.0),
                normal: vec3(0.0, 0.0, 1.0),
                uv: [0.0, index as f32 / count as f32],
            });
            mesh.vertices.push(Vertex {
                position: vec3(c * outer_radius, s * outer_radius, 0.0),
                normal: vec3(0.0, 0.0, 1.0),
                uv: [1.0, index as f32 / count as f32],
            });
        }
        for index in 0..count {
            let a = index * 2;
            let b = a + 1;
            let c = a + 2;
            let d = a + 3;
            mesh.triangles.push([a, c, b]);
            mesh.triangles.push([b, c, d]);
        }
        mesh
    }

    pub fn torus(major_radius: f32, minor_radius: f32, major_steps: usize, minor_steps: usize) -> Self {
        let mut mesh = Self::new();
        let around = major_steps.max(12);
        let tube = minor_steps.max(6);
        for i in 0..=around {
            let u = i as f32 / around as f32;
            let a = u * std::f32::consts::TAU;
            let (sa, ca) = a.sin_cos();
            for j in 0..=tube {
                let v = j as f32 / tube as f32;
                let b = v * std::f32::consts::TAU;
                let (sb, cb) = b.sin_cos();
                let radial = major_radius + minor_radius * cb;
                let position = vec3(radial * ca, radial * sa, minor_radius * sb);
                let normal = normalize(vec3(ca * cb, sa * cb, sb));
                mesh.vertices.push(Vertex {
                    position,
                    normal,
                    uv: [u, v],
                });
            }
        }
        for i in 0..around {
            for j in 0..tube {
                let a = i * (tube + 1) + j;
                let b = a + tube + 1;
                mesh.triangles.push([a, b, a + 1]);
                mesh.triangles.push([a + 1, b, b + 1]);
            }
        }
        mesh
    }

    pub fn octahedron() -> Self {
        let top = vec3(0.0, 1.0, 0.0);
        let bottom = vec3(0.0, -1.0, 0.0);
        let left = vec3(-1.0, 0.0, 0.0);
        let right = vec3(1.0, 0.0, 0.0);
        let front = vec3(0.0, 0.0, 1.0);
        let back = vec3(0.0, 0.0, -1.0);
        let faces = [
            [top, front, right], [top, right, back], [top, back, left], [top, left, front],
            [bottom, right, front], [bottom, back, right], [bottom, left, back], [bottom, front, left],
        ];
        let mut mesh = Self::new();
        for face in faces {
            mesh.add_flat_triangle(face[0], face[1], face[2]);
        }
        mesh
    }

    pub fn cube() -> Self {
        let mut mesh = Self::new();
        let p = [
            vec3(-1.0, -1.0, -1.0), vec3(1.0, -1.0, -1.0),
            vec3(1.0, 1.0, -1.0), vec3(-1.0, 1.0, -1.0),
            vec3(-1.0, -1.0, 1.0), vec3(1.0, -1.0, 1.0),
            vec3(1.0, 1.0, 1.0), vec3(-1.0, 1.0, 1.0),
        ];
        let faces = [
            [0, 2, 1], [0, 3, 2], [4, 5, 6], [4, 6, 7],
            [0, 1, 5], [0, 5, 4], [3, 7, 6], [3, 6, 2],
            [0, 4, 7], [0, 7, 3], [1, 2, 6], [1, 6, 5],
        ];
        for face in faces {
            mesh.add_flat_triangle(p[face[0]], p[face[1]], p[face[2]]);
        }
        mesh
    }

    pub fn spaceship() -> Self {
        let mut mesh = Self::new();
        let nose = vec3(0.0, 0.0, -4.8);
        let top_left = vec3(-0.62, 0.42, 1.6);
        let top_right = vec3(0.62, 0.42, 1.6);
        let bottom_left = vec3(-0.62, -0.34, 1.6);
        let bottom_right = vec3(0.62, -0.34, 1.6);
        let rear_left = vec3(-0.55, 0.0, 2.8);
        let rear_right = vec3(0.55, 0.0, 2.8);

        // Central hull and pointed cockpit.
        mesh.add_flat_triangle(nose, top_left, top_right);
        mesh.add_flat_triangle(nose, top_right, bottom_right);
        mesh.add_flat_triangle(nose, bottom_right, bottom_left);
        mesh.add_flat_triangle(nose, bottom_left, top_left);
        mesh.add_flat_triangle(top_left, rear_left, rear_right);
        mesh.add_flat_triangle(top_left, rear_right, top_right);
        mesh.add_flat_triangle(bottom_left, bottom_right, rear_right);
        mesh.add_flat_triangle(bottom_left, rear_right, rear_left);
        mesh.add_flat_triangle(top_left, bottom_left, rear_left);
        mesh.add_flat_triangle(top_right, rear_right, bottom_right);

        // Broad delta wings.
        mesh.add_flat_triangle(top_left, vec3(-4.5, 0.10, 2.15), vec3(-3.45, 0.10, 3.0));
        mesh.add_flat_triangle(top_left, vec3(-3.45, 0.10, 3.0), rear_left);
        mesh.add_flat_triangle(bottom_left, rear_left, vec3(-3.45, 0.10, 3.0));
        mesh.add_flat_triangle(bottom_left, vec3(-3.45, 0.10, 3.0), vec3(-4.5, 0.10, 2.15));
        mesh.add_flat_triangle(top_right, rear_right, vec3(3.45, 0.10, 3.0));
        mesh.add_flat_triangle(top_right, vec3(3.45, 0.10, 3.0), vec3(4.5, 0.10, 2.15));
        mesh.add_flat_triangle(bottom_right, vec3(4.5, 0.10, 2.15), vec3(3.45, 0.10, 3.0));
        mesh.add_flat_triangle(bottom_right, vec3(3.45, 0.10, 3.0), rear_right);

        // Tail plane and cockpit canopy.
        mesh.add_flat_triangle(vec3(0.0, 0.42, 1.3), vec3(-0.24, 0.8, 2.0), vec3(0.24, 0.8, 2.0));
        mesh.add_flat_triangle(nose, vec3(-0.28, 0.40, 0.2), vec3(0.28, 0.40, 0.2));
        mesh
    }

    pub fn add_flat_triangle(&mut self, a: Vec3, b: Vec3, c: Vec3) {
        let normal = normalize(cross(b - a, c - a));
        let first = self.vertices.len();
        self.vertices.push(Vertex { position: a, normal, uv: [0.0, 0.0] });
        self.vertices.push(Vertex { position: b, normal, uv: [1.0, 0.0] });
        self.vertices.push(Vertex { position: c, normal, uv: [0.5, 1.0] });
        self.triangles.push([first, first + 1, first + 2]);
    }
}

/// Load an OBJ file into independent render vertices. Positive and negative
/// OBJ indices are accepted; missing UVs/normals are generated safely.
pub fn load_obj<P: AsRef<Path>>(path: P) -> Result<Mesh, String> {
    let source = fs::read_to_string(path.as_ref())
        .map_err(|error| format!("No se pudo leer {}: {error}", path.as_ref().display()))?;

    let mut positions: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut texcoords: Vec<[f32; 2]> = Vec::new();
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut has_normal: Vec<bool> = Vec::new();
    let mut triangles: Vec<[usize; 3]> = Vec::new();
    let mut vertex_map: HashMap<(i32, i32, i32), usize> = HashMap::new();

    for (line_number, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else { continue };
        match kind {
            "v" => {
                let values = parse_floats(parts, 3, line_number)?;
                positions.push(vec3(values[0], values[1], values[2]));
            }
            "vn" => {
                let values = parse_floats(parts, 3, line_number)?;
                normals.push(normalize(vec3(values[0], values[1], values[2])));
            }
            "vt" => {
                let values = parse_floats(parts, 2, line_number)?;
                texcoords.push([values[0], 1.0 - values[1]]);
            }
            "f" => {
                let face_tokens: Vec<&str> = parts.collect();
                if face_tokens.len() < 3 {
                    continue;
                }
                let mut face_indices = Vec::with_capacity(face_tokens.len());
                for token in face_tokens {
                    let mut pieces = token.split('/');
                    let position_index = parse_obj_index(pieces.next(), positions.len(), line_number)?;
                    let texcoord_index = parse_obj_index_optional(pieces.next(), texcoords.len(), line_number)?;
                    let normal_index = parse_obj_index_optional(pieces.next(), normals.len(), line_number)?;
                    let key = (
                        position_index as i32,
                        texcoord_index.map(|index| index as i32).unwrap_or(-1),
                        normal_index.map(|index| index as i32).unwrap_or(-1),
                    );
                    let vertex_index = if let Some(existing) = vertex_map.get(&key) {
                        *existing
                    } else {
                        let position = positions[position_index];
                        let uv = texcoord_index
                            .and_then(|index| texcoords.get(index).copied())
                            .unwrap_or([0.0, 0.0]);
                        let has_vertex_normal = normal_index.is_some();
                        let normal = normal_index
                            .and_then(|index| normals.get(index).copied())
                            .unwrap_or(vec3(0.0, 0.0, 0.0));
                        let index = vertices.len();
                        vertices.push(Vertex { position, normal, uv });
                        has_normal.push(has_vertex_normal);
                        vertex_map.insert(key, index);
                        index
                    };
                    face_indices.push(vertex_index);
                }
                for index in 1..face_indices.len() - 1 {
                    triangles.push([face_indices[0], face_indices[index], face_indices[index + 1]]);
                }
            }
            _ => {
                // mtllib/usemtl and object/group labels are intentionally not
                // needed by this small importer; materials are selected by the game.
            }
        }
    }

    if positions.is_empty() || triangles.is_empty() {
        return Err("El OBJ no contiene vértices y caras triangulables.".to_string());
    }

    // Generate vertex normals for any face that did not supply `vn` data.
    let mut generated = vec![vec3(0.0, 0.0, 0.0); vertices.len()];
    for triangle in &triangles {
        let a = vertices[triangle[0]].position;
        let b = vertices[triangle[1]].position;
        let c = vertices[triangle[2]].position;
        let face_normal = normalize(cross(b - a, c - a));
        for index in triangle {
            if !has_normal[*index] {
                generated[*index] += face_normal;
            }
        }
    }
    for index in 0..vertices.len() {
        if !has_normal[index] {
            vertices[index].normal = normalize(generated[index]);
        }
    }

    Ok(Mesh { vertices, triangles })
}

fn parse_floats<'a>(mut values: impl Iterator<Item = &'a str>, count: usize, line: usize) -> Result<Vec<f32>, String> {
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let value = values
            .next()
            .ok_or_else(|| format!("Faltan componentes numéricos en OBJ, línea {}", line + 1))?;
        result.push(value.parse::<f32>().map_err(|_| {
            format!("Componente numérico inválido en OBJ, línea {}", line + 1)
        })?);
    }
    Ok(result)
}

fn parse_obj_index(value: Option<&str>, length: usize, line: usize) -> Result<usize, String> {
    let raw = value
        .ok_or_else(|| format!("Índice de posición ausente en OBJ, línea {}", line + 1))?;
    let number = raw
        .parse::<i32>()
        .map_err(|_| format!("Índice OBJ inválido en línea {}", line + 1))?;
    resolve_obj_index(number, length, line)
}

fn parse_obj_index_optional(value: Option<&str>, length: usize, line: usize) -> Result<Option<usize>, String> {
    let Some(raw) = value.filter(|part| !part.is_empty()) else { return Ok(None) };
    let number = raw
        .parse::<i32>()
        .map_err(|_| format!("Índice OBJ inválido en línea {}", line + 1))?;
    Ok(Some(resolve_obj_index(number, length, line)?))
}

fn resolve_obj_index(number: i32, length: usize, line: usize) -> Result<usize, String> {
    let index = if number > 0 {
        number - 1
    } else if number < 0 {
        length as i32 + number
    } else {
        -1
    };
    if index < 0 || index as usize >= length {
        Err(format!("Índice OBJ fuera de rango en línea {}", line + 1))
    } else {
        Ok(index as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_sphere_has_triangles() {
        let sphere = Mesh::uv_sphere(8, 12);
        assert!(!sphere.vertices.is_empty());
        assert_eq!(sphere.triangles.len(), 8 * 12 * 2);
    }

    #[test]
    fn flat_triangle_creates_a_normal() {
        let mut mesh = Mesh::new();
        mesh.add_flat_triangle(
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
        );
        assert!(mesh.vertices[0].normal.z > 0.99);
    }

    #[test]
    fn supplied_obj_asset_can_be_loaded() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/models/esfera.obj");
        let mesh = load_obj(path).expect("the supplied OBJ test model should load");
        assert_eq!(mesh.triangles.len(), 320);
        assert!(!mesh.vertices.is_empty());
    }
}
