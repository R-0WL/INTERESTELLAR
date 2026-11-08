//! CPU triangle rasterizer.
//!
//! This module owns the complete rendering path:
//! model transform -> view transform -> perspective projection -> viewport
//! -> barycentric triangle fill -> depth test -> fragment/material function.
//! No GPU rendering API or scene-graph engine is used.

use crate::framebuffer::{color_from_f32, Framebuffer};
use crate::game::{Game, PlanetKind, SpaceBody};
use crate::math::{distance, dot, length, normalize, vec3, Mat4, Vec3};
use crate::mesh::Mesh;

#[derive(Clone, Copy)]
enum ShaderKind {
    Solid,
    Earth,
    Moon,
    Mars,
    Jupiter,
    Saturn,
    Miller,
    Mann,
    Edmund,
    Sun,
    Ring,
    Wormhole,
    AccretionDisc,
    Rock,
    Water,
    IceGround,
    DesertGround,
    GreenGround,
    GreyGround,
}

#[derive(Clone, Copy)]
struct Material {
    base_color: Vec3,
    shader: ShaderKind,
    emissive: f32,
    specular: f32,
}

impl Material {
    fn new(color: Vec3, shader: ShaderKind) -> Self {
        Self { base_color: color, shader, emissive: 0.0, specular: 0.18 }
    }

    fn emissive(mut self, amount: f32) -> Self {
        self.emissive = amount;
        self
    }

    fn specular(mut self, amount: f32) -> Self {
        self.specular = amount;
        self
    }
}

#[derive(Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    z: f32,
    inv_w: f32,
    world_over_w: Vec3,
    normal_over_w: Vec3,
    uv_over_w: [f32; 2],
}

#[derive(Clone, Copy)]
struct Star {
    direction: Vec3,
    color: u32,
    radius: i32,
}

pub struct Renderer {
    width: usize,
    height: usize,
    sphere: Mesh,
    ship: Mesh,
    cube: Mesh,
    rock: Mesh,
    torus: Mesh,
    ring: Mesh,
    ground: Mesh,
    stars: Vec<Star>,
    asteroids: Vec<(Vec3, f32, f32)>,
}

impl Renderer {
    pub fn new(width: usize, height: usize, imported_sphere: Option<Mesh>) -> Self {
        let sphere = imported_sphere.unwrap_or_else(|| Mesh::uv_sphere(18, 28));
        let stars = make_stars(1500);
        let asteroids = make_asteroids();
        Self {
            width,
            height,
            sphere,
            ship: Mesh::spaceship(),
            cube: Mesh::cube(),
            rock: Mesh::octahedron(),
            torus: Mesh::torus(1.0, 0.12, 64, 10),
            ring: Mesh::ring(1.0, 2.5, 96),
            ground: Mesh::plane_xz(180.0, 180.0, 32, 0.0),
            stars,
            asteroids,
        }
    }

    pub fn render(&self, framebuffer: &mut Framebuffer, game: &Game) {
        framebuffer.clear_vertical_gradient(0x02040B, 0x080D1A);
        let (eye, target) = game.camera();
        let view = Mat4::look_at(eye, target, vec3(0.0, 1.0, 0.0));
        let projection = Mat4::perspective(
            62.0_f32.to_radians(),
            self.width as f32 / self.height as f32,
            0.1,
            24000.0,
        );
        let view_projection = projection * view;

        if let Some(body_index) = game.landed_body {
            let body = &game.bodies[body_index];
            self.render_landing(framebuffer, game, body, eye, view_projection);
        } else {
            self.render_stars(framebuffer, eye, view_projection);
            self.render_space_bodies(framebuffer, game, eye, view_projection);
            if !game.destroyed {
                self.render_ship(framebuffer, game, eye, view_projection);
            }
        }
    }

    fn render_stars(&self, framebuffer: &mut Framebuffer, eye: Vec3, view_projection: Mat4) {
        for star in &self.stars {
            // Place each star far from the camera along a fixed direction. This
            // keeps the background stable while avoiding an enormous world mesh.
            let position = eye + star.direction * 14500.0;
            if let Some((x, y, depth)) = project_point(position, view_projection, self.width, self.height) {
                if star.radius <= 1 {
                    framebuffer.set_pixel_depth(x, y, depth, star.color);
                } else {
                    for oy in -star.radius..=star.radius {
                        for ox in -star.radius..=star.radius {
                            if ox * ox + oy * oy <= star.radius * star.radius {
                                framebuffer.set_pixel_depth(x + ox, y + oy, depth, star.color);
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_space_bodies(&self, framebuffer: &mut Framebuffer, game: &Game, eye: Vec3, vp: Mat4) {
        for body in &game.bodies {
            if !game.body_is_visible(body) {
                continue;
            }
            if distance(body.position, eye) > 23000.0 {
                continue;
            }
            self.render_body(framebuffer, body, game.clock, eye, vp);
        }

        if !game.crossed_wormhole {
            if let Some(asteroid_center) = game.bodies.iter().find(|body| body.kind == PlanetKind::AsteroidBelt) {
                if distance(asteroid_center.position, eye) < 2500.0 {
                    self.render_asteroid_belt(framebuffer, asteroid_center.position, eye, vp);
                }
            }
        }
    }

    fn render_body(&self, fb: &mut Framebuffer, body: &SpaceBody, time: f32, eye: Vec3, vp: Mat4) {
        let position = body.position;
        let radius = body.radius;
        match body.kind {
            PlanetKind::Earth => {
                let model = Mat4::translation(position)
                    * Mat4::rotation_y(time * 0.018)
                    * Mat4::scale(vec3(radius, radius, radius));
                self.draw_mesh(fb, &self.sphere, model, material_for(body.kind), eye, vp, time);
                let atmosphere = Mat4::translation(position)
                    * Mat4::rotation_x(0.18)
                    * Mat4::scale(vec3(radius * 1.08, radius * 1.08, radius * 0.045));
                self.draw_mesh(fb, &self.torus, atmosphere, Material::new(vec3(0.27, 0.65, 1.0), ShaderKind::Wormhole).emissive(0.16), eye, vp, time);
            }
            PlanetKind::Sun => {
                let model = Mat4::translation(position) * Mat4::scale(vec3(radius, radius, radius));
                self.draw_mesh(fb, &self.sphere, model, Material::new(vec3(1.0, 0.68, 0.25), ShaderKind::Sun).emissive(1.8), eye, vp, time);
                let halo = Mat4::translation(position)
                    * Mat4::rotation_x(0.28)
                    * Mat4::rotation_z(time * 0.02)
                    * Mat4::scale(vec3(radius * 1.12, radius * 1.12, radius * 0.035));
                self.draw_mesh(fb, &self.torus, halo, Material::new(vec3(1.0, 0.38, 0.12), ShaderKind::Sun).emissive(0.22), eye, vp, time);
            }
            PlanetKind::Moon | PlanetKind::Mars | PlanetKind::Jupiter | PlanetKind::MillerWater | PlanetKind::MannIce | PlanetKind::EdmundHabitable => {
                let rotation = Mat4::rotation_y(time * if body.kind == PlanetKind::Jupiter { 0.035 } else { 0.012 });
                let model = Mat4::translation(position) * rotation * Mat4::scale(vec3(radius, radius, radius));
                self.draw_mesh(fb, &self.sphere, model, material_for(body.kind), eye, vp, time);
            }
            PlanetKind::Saturn => {
                let model = Mat4::translation(position)
                    * Mat4::rotation_y(time * 0.012)
                    * Mat4::scale(vec3(radius, radius, radius));
                self.draw_mesh(fb, &self.sphere, model, material_for(body.kind), eye, vp, time);
                let rings = Mat4::translation(position)
                    * Mat4::rotation_x(1.13)
                    * Mat4::rotation_z(0.14)
                    * Mat4::scale(vec3(radius * 1.0, radius * 1.0, radius * 0.16));
                self.draw_mesh(fb, &self.ring, rings, Material::new(vec3(0.72, 0.61, 0.43), ShaderKind::Ring).specular(0.35), eye, vp, time);
            }
            PlanetKind::Wormhole => {
                for index in 0..7 {
                    let scale = 1.0 + index as f32 * 0.26;
                    let model = Mat4::translation(position)
                        * Mat4::rotation_z(time * (0.10 + index as f32 * 0.012))
                        * Mat4::scale(vec3(radius * scale, radius * scale, radius * 0.22));
                    let color = if index % 2 == 0 { vec3(0.30, 0.78, 1.0) } else { vec3(0.82, 0.57, 1.0) };
                    self.draw_mesh(fb, &self.torus, model, Material::new(color, ShaderKind::Wormhole).emissive(0.7), eye, vp, time);
                }
                let glow = Mat4::translation(position) * Mat4::scale(vec3(radius * 0.55, radius * 0.55, radius * 0.55));
                self.draw_mesh(fb, &self.sphere, glow, Material::new(vec3(0.08, 0.2, 0.42), ShaderKind::Wormhole).emissive(0.5), eye, vp, time);
            }
            PlanetKind::Docking => {
                let model = Mat4::translation(position) * Mat4::rotation_z(time * 0.08) * Mat4::scale(vec3(radius * 1.4, radius * 1.4, radius * 0.3));
                self.draw_mesh(fb, &self.torus, model, Material::new(vec3(0.62, 0.72, 0.78), ShaderKind::Solid).specular(0.75), eye, vp, time);
            }
            PlanetKind::Gargantua => {
                let black_hole = Mat4::translation(position) * Mat4::scale(vec3(radius, radius, radius));
                self.draw_mesh(fb, &self.sphere, black_hole, Material::new(vec3(0.004, 0.003, 0.008), ShaderKind::Solid), eye, vp, time);

                let disk = Mat4::translation(position)
                    * Mat4::rotation_x(0.32)
                    * Mat4::rotation_z(time * 0.04)
                    * Mat4::scale(vec3(radius * 1.35, radius * 1.35, radius * 0.08));
                self.draw_mesh(fb, &self.ring, disk, Material::new(vec3(1.0, 0.47, 0.17), ShaderKind::AccretionDisc).emissive(0.85), eye, vp, time);

                let outer = Mat4::translation(position)
                    * Mat4::rotation_x(0.32)
                    * Mat4::scale(vec3(radius * 1.95, radius * 1.95, radius * 0.05));
                self.draw_mesh(fb, &self.ring, outer, Material::new(vec3(0.35, 0.24, 0.62), ShaderKind::AccretionDisc).emissive(0.18), eye, vp, time);
            }
            PlanetKind::AsteroidBelt => {}
        }
    }

    fn render_asteroid_belt(&self, fb: &mut Framebuffer, center: Vec3, eye: Vec3, vp: Mat4) {
        for (offset, size, rotation) in &self.asteroids {
            let position = center + *offset;
            if distance(position, eye) > 1700.0 {
                continue;
            }
            let model = Mat4::translation(position)
                * Mat4::rotation_y(*rotation)
                * Mat4::rotation_x(*rotation * 0.7)
                * Mat4::scale(vec3(*size, *size * 0.75, *size * 1.2));
            self.draw_mesh(fb, &self.rock, model, Material::new(vec3(0.37, 0.34, 0.31), ShaderKind::Rock), eye, vp, 0.0);
        }
    }

    fn render_ship(&self, fb: &mut Framebuffer, game: &Game, eye: Vec3, vp: Mat4) {
        let model = Mat4::translation(game.position)
            * Mat4::rotation_y(game.yaw)
            * Mat4::rotation_x(game.pitch)
            * Mat4::scale(vec3(1.0, 1.0, 1.0));
        self.draw_mesh(fb, &self.ship, model, Material::new(vec3(0.72, 0.79, 0.83), ShaderKind::Solid).specular(0.82), eye, vp, game.clock);

        // A pair of small, bright engine plumes behind the tail.
        let right = game.right();
        let up = game.up();
        let backward = -game.forward();
        for side in [-0.42, 0.42] {
            let center = game.position + right * side + backward * 3.15;
            let exhaust_model = Mat4::translation(center)
                * Mat4::rotation_y(game.yaw)
                * Mat4::rotation_x(game.pitch)
                * Mat4::scale(vec3(0.12, 0.12, if game.speed > 180.0 { 1.15 } else { 0.55 }));
            let _ = up; // Kept explicit to show the ship's local basis.
            self.draw_mesh(fb, &self.torus, exhaust_model, Material::new(vec3(0.20, 0.78, 1.0), ShaderKind::Wormhole).emissive(1.0), eye, vp, game.clock);
        }
    }

    fn render_landing(&self, fb: &mut Framebuffer, game: &Game, body: &SpaceBody, eye: Vec3, vp: Mat4) {
        let (top, bottom, ground_color, ground_shader) = match body.kind {
            PlanetKind::MillerWater => (0x4A9CBD, 0xC0E9E8, vec3(0.04, 0.28, 0.43), ShaderKind::Water),
            PlanetKind::MannIce => (0x8EAFC9, 0xE3F1F8, vec3(0.72, 0.82, 0.88), ShaderKind::IceGround),
            PlanetKind::Mars => (0xB65F40, 0xE4A16C, vec3(0.55, 0.25, 0.14), ShaderKind::DesertGround),
            PlanetKind::Moon => (0x111A25, 0x69747E, vec3(0.34, 0.36, 0.37), ShaderKind::GreyGround),
            PlanetKind::EdmundHabitable => (0x769D9A, 0xE5C89A, vec3(0.16, 0.31, 0.18), ShaderKind::GreenGround),
            _ => (0x234B78, 0x87BFEA, vec3(0.10, 0.28, 0.48), ShaderKind::Water),
        };
        fb.clear_vertical_gradient(top, bottom);

        let anchor = game.position;
        let ground_model = Mat4::translation(anchor + vec3(0.0, -0.6, -24.0));
        self.draw_mesh(fb, &self.ground, ground_model, Material::new(ground_color, ground_shader), eye, vp, game.clock);

        match body.kind {
            PlanetKind::MillerWater => self.draw_miller_surface(fb, anchor, eye, vp, game.clock),
            PlanetKind::MannIce => self.draw_ice_surface(fb, anchor, eye, vp),
            PlanetKind::EdmundHabitable => self.draw_green_surface(fb, anchor, eye, vp),
            PlanetKind::Mars => self.draw_rocky_surface(fb, anchor, eye, vp, vec3(0.55, 0.24, 0.13)),
            PlanetKind::Moon => self.draw_rocky_surface(fb, anchor, eye, vp, vec3(0.43, 0.46, 0.47)),
            PlanetKind::Earth => self.draw_green_surface(fb, anchor, eye, vp),
            _ => {}
        }

        // The Miller vignette retains the distant black hole in the sky.
        if body.kind == PlanetKind::MillerWater {
            let center = anchor + vec3(38.0, 30.0, -82.0);
            let hole = Mat4::translation(center) * Mat4::scale(vec3(12.0, 12.0, 12.0));
            self.draw_mesh(fb, &self.sphere, hole, Material::new(vec3(0.002, 0.002, 0.004), ShaderKind::Solid), eye, vp, game.clock);
            let disk = Mat4::translation(center) * Mat4::rotation_x(0.4) * Mat4::scale(vec3(20.0, 20.0, 0.7));
            self.draw_mesh(fb, &self.ring, disk, Material::new(vec3(1.0, 0.58, 0.26), ShaderKind::AccretionDisc).emissive(0.85), eye, vp, game.clock);
        }
    }

    fn draw_miller_surface(&self, fb: &mut Framebuffer, anchor: Vec3, eye: Vec3, vp: Mat4, time: f32) {
        let wave = Mesh::cube();
        for index in 0..7 {
            let x = -45.0 + index as f32 * 15.0;
            let z = -54.0 - (index % 3) as f32 * 8.0;
            let y = 0.9 + (time * 1.8 + index as f32).sin() * 0.48;
            let model = Mat4::translation(anchor + vec3(x, y, z))
                * Mat4::rotation_y(0.03 * index as f32)
                * Mat4::scale(vec3(7.0 + (index % 3) as f32 * 2.0, 1.2, 0.7));
            self.draw_mesh(fb, &wave, model, Material::new(vec3(0.55, 0.88, 0.95), ShaderKind::Water).emissive(0.12), eye, vp, time);
        }
        let foam = Mat4::translation(anchor + vec3(0.0, 0.08, -34.0)) * Mat4::scale(vec3(72.0, 0.02, 72.0));
        self.draw_mesh(fb, &self.ring, foam, Material::new(vec3(0.46, 0.9, 0.98), ShaderKind::Water).emissive(0.12), eye, vp, time);
    }

    fn draw_ice_surface(&self, fb: &mut Framebuffer, anchor: Vec3, eye: Vec3, vp: Mat4) {
        for index in 0..18 {
            let x = ((index * 17) % 74) as f32 - 37.0;
            let z = -15.0 - ((index * 23) % 56) as f32;
            let scale = 1.5 + (index % 4) as f32 * 0.65;
            let model = Mat4::translation(anchor + vec3(x, 1.5 + scale, z))
                * Mat4::rotation_z((index as f32 * 0.73).sin() * 0.22)
                * Mat4::scale(vec3(scale, scale * 1.8, scale));
            self.draw_mesh(fb, &self.rock, model, Material::new(vec3(0.74, 0.84, 0.91), ShaderKind::IceGround), eye, vp, 0.0);
        }
    }

    fn draw_green_surface(&self, fb: &mut Framebuffer, anchor: Vec3, eye: Vec3, vp: Mat4) {
        for index in 0..22 {
            let x = ((index * 13) % 72) as f32 - 36.0;
            let z = -13.0 - ((index * 19) % 62) as f32;
            let scale = 1.2 + (index % 4) as f32 * 0.5;
            let model = Mat4::translation(anchor + vec3(x, scale * 0.55, z))
                * Mat4::rotation_y(index as f32 * 0.61)
                * Mat4::scale(vec3(scale, scale * 0.7, scale * 0.8));
            self.draw_mesh(fb, &self.rock, model, Material::new(vec3(0.25, 0.38, 0.21), ShaderKind::GreenGround), eye, vp, 0.0);
        }
        for x in [-20.0, -5.0, 13.0, 28.0] {
            let tree = Mat4::translation(anchor + vec3(x, 2.3, -48.0)) * Mat4::scale(vec3(0.32, 3.0, 0.32));
            self.draw_mesh(fb, &self.cube, tree, Material::new(vec3(0.17, 0.34, 0.19), ShaderKind::GreenGround), eye, vp, 0.0);
        }
    }

    fn draw_rocky_surface(&self, fb: &mut Framebuffer, anchor: Vec3, eye: Vec3, vp: Mat4, color: Vec3) {
        for index in 0..28 {
            let x = ((index * 17) % 82) as f32 - 41.0;
            let z = -12.0 - ((index * 11) % 64) as f32;
            let scale = 0.8 + (index % 5) as f32 * 0.55;
            let model = Mat4::translation(anchor + vec3(x, scale * 0.5, z))
                * Mat4::rotation_y(index as f32 * 0.83)
                * Mat4::scale(vec3(scale * 1.2, scale * 0.7, scale));
            self.draw_mesh(fb, &self.rock, model, Material::new(color, ShaderKind::Rock), eye, vp, 0.0);
        }
    }

    /// Transform and rasterize a complete indexed triangle mesh.
    fn draw_mesh(
        &self,
        fb: &mut Framebuffer,
        mesh: &Mesh,
        model: Mat4,
        material: Material,
        eye: Vec3,
        vp: Mat4,
        time: f32,
    ) {
        if mesh.vertices.is_empty() || mesh.triangles.is_empty() {
            return;
        }
        let mvp = vp * model;
        let mut transformed: Vec<Option<ScreenVertex>> = Vec::with_capacity(mesh.vertices.len());
        for vertex in &mesh.vertices {
            let world = model.transform_point(vertex.position);
            let normal = model.transform_direction(vertex.normal);
            let clip = mvp.transform_vec4([vertex.position.x, vertex.position.y, vertex.position.z, 1.0]);
            if clip[3] <= 0.1 || clip[3].is_nan() {
                transformed.push(None);
                continue;
            }
            let inv_w = 1.0 / clip[3];
            let ndc_x = clip[0] * inv_w;
            let ndc_y = clip[1] * inv_w;
            let ndc_z = clip[2] * inv_w;
            let screen_x = (ndc_x * 0.5 + 0.5) * self.width as f32;
            let screen_y = (0.5 - ndc_y * 0.5) * self.height as f32;
            transformed.push(Some(ScreenVertex {
                x: screen_x,
                y: screen_y,
                z: ndc_z,
                inv_w,
                world_over_w: world * inv_w,
                normal_over_w: normal * inv_w,
                uv_over_w: [vertex.uv[0] * inv_w, vertex.uv[1] * inv_w],
            }));
        }

        for triangle in &mesh.triangles {
            let Some(a) = transformed[triangle[0]] else { continue };
            let Some(b) = transformed[triangle[1]] else { continue };
            let Some(c) = transformed[triangle[2]] else { continue };
            self.rasterize_triangle(fb, a, b, c, material, eye, time);
        }
    }

    fn rasterize_triangle(
        &self,
        fb: &mut Framebuffer,
        a: ScreenVertex,
        b: ScreenVertex,
        c: ScreenVertex,
        material: Material,
        eye: Vec3,
        time: f32,
    ) {
        let area = edge(a.x, a.y, b.x, b.y, c.x, c.y);
        if area.abs() < 0.0001 {
            return;
        }

        let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as i32;
        let max_x = a.x.max(b.x).max(c.x).ceil().min((self.width - 1) as f32) as i32;
        let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as i32;
        let max_y = a.y.max(b.y).max(c.y).ceil().min((self.height - 1) as f32) as i32;
        if min_x > max_x || min_y > max_y {
            return;
        }

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let w0 = edge(b.x, b.y, c.x, c.y, px, py) / area;
                let w1 = edge(c.x, c.y, a.x, a.y, px, py) / area;
                let w2 = edge(a.x, a.y, b.x, b.y, px, py) / area;
                if w0 < -0.0001 || w1 < -0.0001 || w2 < -0.0001 {
                    continue;
                }

                let depth = w0 * a.z + w1 * b.z + w2 * c.z;
                if !(0.0..=1.0).contains(&depth) {
                    continue;
                }
                let reciprocal_w = w0 * a.inv_w + w1 * b.inv_w + w2 * c.inv_w;
                if reciprocal_w.abs() < 1.0e-8 {
                    continue;
                }
                let correction = 1.0 / reciprocal_w;
                let world_position = (a.world_over_w * w0 + b.world_over_w * w1 + c.world_over_w * w2) * correction;
                let normal = normalize((a.normal_over_w * w0 + b.normal_over_w * w1 + c.normal_over_w * w2) * correction);
                let uv = [
                    (a.uv_over_w[0] * w0 + b.uv_over_w[0] * w1 + c.uv_over_w[0] * w2) * correction,
                    (a.uv_over_w[1] * w0 + b.uv_over_w[1] * w1 + c.uv_over_w[1] * w2) * correction,
                ];
                let color = self.fragment(material, uv, world_position, normal, eye, time);
                fb.set_pixel_depth(x, y, depth, color);
            }
        }
    }

    /// CPU fragment shader: material pattern + diffuse/specular lighting.
    fn fragment(&self, material: Material, uv: [f32; 2], position: Vec3, normal: Vec3, eye: Vec3, time: f32) -> u32 {
        let n = if length(normal) > 0.1 { normalize(normal) } else { vec3(0.0, 1.0, 0.0) };
        let albedo = sample_material(material, uv, n, position, time);
        let light_direction = normalize(vec3(-0.42, 0.72, 0.55));
        let diffuse = dot(n, light_direction).max(0.0);
        let view_direction = normalize(eye - position);
        let half_vector = normalize(light_direction + view_direction);
        let specular = dot(n, half_vector).max(0.0).powf(26.0) * material.specular;
        let illumination = 0.22 + diffuse * 0.82;
        let emissive = material.emissive;
        let color = albedo * (illumination + emissive) + vec3(specular, specular, specular);
        color_from_f32(color.x, color.y, color.z)
    }
}

fn material_for(kind: PlanetKind) -> Material {
    match kind {
        PlanetKind::Earth => Material::new(vec3(0.22, 0.48, 0.78), ShaderKind::Earth).specular(0.38),
        PlanetKind::Moon => Material::new(vec3(0.55, 0.57, 0.58), ShaderKind::Moon),
        PlanetKind::Mars => Material::new(vec3(0.72, 0.29, 0.14), ShaderKind::Mars),
        PlanetKind::Jupiter => Material::new(vec3(0.72, 0.48, 0.31), ShaderKind::Jupiter),
        PlanetKind::Saturn => Material::new(vec3(0.77, 0.66, 0.45), ShaderKind::Saturn),
        PlanetKind::MillerWater => Material::new(vec3(0.05, 0.39, 0.67), ShaderKind::Miller).specular(0.75),
        PlanetKind::MannIce => Material::new(vec3(0.76, 0.84, 0.91), ShaderKind::Mann).specular(0.4),
        PlanetKind::EdmundHabitable => Material::new(vec3(0.28, 0.47, 0.27), ShaderKind::Edmund),
        _ => Material::new(vec3(0.6, 0.6, 0.6), ShaderKind::Solid),
    }
}

fn sample_material(material: Material, uv: [f32; 2], normal: Vec3, position: Vec3, time: f32) -> Vec3 {
    let u = uv[0];
    let v = uv[1];
    match material.shader {
        ShaderKind::Solid => material.base_color,
        ShaderKind::Earth => {
            let continents = noise3(normal * 5.2) + 0.38 * noise3(normal * 12.0);
            if continents > 0.68 {
                vec3(0.19, 0.39 + normal.y.max(0.0) * 0.10, 0.20)
            } else {
                vec3(0.035, 0.19 + normal.y * 0.035, 0.48 + normal.y.max(0.0) * 0.13)
            }
        }
        ShaderKind::Moon => {
            let craters = noise3(normal * 27.0);
            let shade = 0.72 + craters * 0.5;
            vec3(0.48, 0.50, 0.51) * shade
        }
        ShaderKind::Mars => {
            let variation = noise3(normal * 18.0);
            vec3(0.52 + variation * 0.24, 0.19 + variation * 0.12, 0.085 + variation * 0.05)
        }
        ShaderKind::Jupiter => {
            let band = (v * 78.0 + (u * 9.0).sin() * 1.8).sin();
            let cloud = (band * 0.16 + noise3(normal * 15.0) * 0.10).clamp(-0.3, 0.3);
            let storm_distance = ((u - 0.72).powi(2) * 2.0 + (v - 0.56).powi(2) * 7.0).sqrt();
            if storm_distance < 0.09 {
                vec3(0.72, 0.20, 0.10)
            } else {
                vec3(0.69 + cloud, 0.43 + cloud * 0.8, 0.27 + cloud * 0.6)
            }
        }
        ShaderKind::Saturn => {
            let band = (v * 95.0 + (u * 7.0).sin()).sin() * 0.08;
            vec3(0.72 + band, 0.61 + band, 0.42 + band * 0.65)
        }
        ShaderKind::Miller => {
            let wave = (v * 68.0 + time * 0.04 + (u * 8.0).sin()).sin();
            vec3(0.025, 0.25 + wave.abs() * 0.12, 0.47 + wave * 0.09)
        }
        ShaderKind::Mann => {
            let ice = noise3(normal * 21.0);
            let blue = (ice - 0.5).abs() * 0.16;
            vec3(0.70 + blue, 0.81 + blue, 0.91 + blue)
        }
        ShaderKind::Edmund => {
            let variation = noise3(normal * 13.0);
            if variation > 0.58 {
                vec3(0.38, 0.45, 0.22)
            } else {
                vec3(0.16, 0.36 + variation * 0.08, 0.19)
            }
        }
        ShaderKind::Sun => {
            let pulse = 0.92 + 0.08 * (time * 1.7 + v * 12.0).sin();
            vec3(1.0, 0.58 * pulse, 0.16 * pulse)
        }
        ShaderKind::Ring => {
            let stripe = (v * 90.0).sin() * 0.08;
            let radius_pattern = (u * 9.0).sin() * 0.04;
            vec3(0.73 + stripe, 0.63 + stripe + radius_pattern, 0.47 + stripe)
        }
        ShaderKind::Wormhole => {
            let pulse = 0.72 + 0.28 * (time * 2.4 + u * 18.0).sin().abs();
            vec3(0.20 * pulse, 0.68 * pulse, 1.0 * pulse)
        }
        ShaderKind::AccretionDisc => {
            let band = (v * 42.0 + time * 0.3 + u * 6.0).sin();
            let heat = (0.76 + band * 0.18).clamp(0.2, 1.0);
            vec3(1.0 * heat, 0.34 + 0.34 * heat, 0.10 + 0.18 * heat)
        }
        ShaderKind::Rock => {
            let variation = noise3(position * 0.055);
            material.base_color * (0.68 + variation * 0.62)
        }
        ShaderKind::Water => {
            let wave = (position.x * 0.08 + position.z * 0.04 + time * 1.4).sin();
            vec3(0.025, 0.24 + wave.abs() * 0.08, 0.39 + wave.abs() * 0.11)
        }
        ShaderKind::IceGround => {
            let crack = (position.x * 0.09 + (position.z * 0.13).sin() * 2.0).sin().abs();
            vec3(0.66 + crack * 0.22, 0.78 + crack * 0.17, 0.88 + crack * 0.11)
        }
        ShaderKind::DesertGround => {
            let dunes = (position.x * 0.04 + position.z * 0.025).sin() * 0.08;
            vec3(0.53 + dunes, 0.23 + dunes * 0.6, 0.12 + dunes * 0.4)
        }
        ShaderKind::GreenGround => {
            let patch = noise3(position * 0.12);
            vec3(0.12 + patch * 0.12, 0.27 + patch * 0.2, 0.12 + patch * 0.10)
        }
        ShaderKind::GreyGround => {
            let crater = noise3(position * 0.16);
            vec3(0.30 + crater * 0.18, 0.33 + crater * 0.18, 0.35 + crater * 0.18)
        }
    }
}

fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (px - ax) * (by - ay) - (py - ay) * (bx - ax)
}

fn project_point(point: Vec3, vp: Mat4, width: usize, height: usize) -> Option<(i32, i32, f32)> {
    let clip = vp.transform_vec4([point.x, point.y, point.z, 1.0]);
    if clip[3] <= 0.1 {
        return None;
    }
    let inv_w = 1.0 / clip[3];
    let ndc_x = clip[0] * inv_w;
    let ndc_y = clip[1] * inv_w;
    let ndc_z = clip[2] * inv_w;
    if !(-1.2..=1.2).contains(&ndc_x) || !(-1.2..=1.2).contains(&ndc_y) {
        return None;
    }
    let x = ((ndc_x * 0.5 + 0.5) * width as f32) as i32;
    let y = ((0.5 - ndc_y * 0.5) * height as f32) as i32;
    Some((x, y, ndc_z))
}

fn make_stars(count: usize) -> Vec<Star> {
    let mut rng = SimpleRng::new(1901);
    let mut stars = Vec::with_capacity(count);
    for _ in 0..count {
        let z = rng.range(-1.0, 1.0);
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let radial = (1.0 - z * z).max(0.0).sqrt();
        let direction = vec3(radial * angle.cos(), z, radial * angle.sin());
        let roll = rng.next();
        let color = if roll > 0.94 {
            0xBBDFFF
        } else if roll > 0.78 {
            0xFFE7C3
        } else {
            0xFFFFFF
        };
        stars.push(Star {
            direction,
            color,
            radius: if rng.next() > 0.985 { 2 } else { 1 },
        });
    }
    stars
}

fn make_asteroids() -> Vec<(Vec3, f32, f32)> {
    let mut rng = SimpleRng::new(7331);
    let mut rocks = Vec::with_capacity(170);
    for _ in 0..170 {
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let radius = rng.range(125.0, 250.0);
        let y = rng.range(-78.0, 78.0);
        let position = vec3(angle.cos() * radius, y, angle.sin() * radius);
        let size = rng.range(2.0, 8.0);
        let rotation = rng.range(0.0, std::f32::consts::TAU);
        rocks.push((position, size, rotation));
    }
    rocks
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self { Self { state: seed.max(1) } }

    fn next(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state as u32) as f32 / u32::MAX as f32
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next()
    }
}

fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let mut n = x.wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(z.wrapping_mul(2147483647));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    let value = n ^ (n >> 16);
    (value as u32) as f32 / u32::MAX as f32
}

fn fade(value: f32) -> f32 { value * value * (3.0 - 2.0 * value) }

fn noise3(point: Vec3) -> f32 {
    let x0 = point.x.floor() as i32;
    let y0 = point.y.floor() as i32;
    let z0 = point.z.floor() as i32;
    let fx = fade(point.x - x0 as f32);
    let fy = fade(point.y - y0 as f32);
    let fz = fade(point.z - z0 as f32);

    let c000 = hash3(x0, y0, z0);
    let c100 = hash3(x0 + 1, y0, z0);
    let c010 = hash3(x0, y0 + 1, z0);
    let c110 = hash3(x0 + 1, y0 + 1, z0);
    let c001 = hash3(x0, y0, z0 + 1);
    let c101 = hash3(x0 + 1, y0, z0 + 1);
    let c011 = hash3(x0, y0 + 1, z0 + 1);
    let c111 = hash3(x0 + 1, y0 + 1, z0 + 1);

    let x00 = c000 + (c100 - c000) * fx;
    let x10 = c010 + (c110 - c010) * fx;
    let x01 = c001 + (c101 - c001) * fx;
    let x11 = c011 + (c111 - c011) * fx;
    let y0_value = x00 + (x10 - x00) * fy;
    let y1_value = x01 + (x11 - x01) * fy;
    y0_value + (y1_value - y0_value) * fz
}
