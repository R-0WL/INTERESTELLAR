//! CPU triangle rasterizer.
//!
//! This module owns the complete rendering path:
//! model transform -> view transform -> perspective projection -> viewport
//! -> barycentric triangle fill -> depth test -> fragment/material function.
//! No GPU rendering API or scene-graph engine is used.

use crate::framebuffer::{color_from_f32, Framebuffer};
use crate::game::{Game, PlanetKind, SpaceBody, AU_SCALE, ASTEROID_BELT_INNER_AU, ASTEROID_BELT_OUTER_AU, GARGANTUA_DISK_RADIUS, GARGANTUA_RADIUS};
use crate::math::{cross, distance, dot, length, normalize, vec3, Mat4, Vec3};
use crate::mesh::Mesh;

#[derive(Clone, Copy)]
enum ShaderKind {
    Solid,
    Mercury,
    Venus,
    Earth,
    Moon,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
    Miller,
    Mann,
    Edmund,
    Sun,
    Ring,
    Wormhole,
    AccretionDisc,
    Rock,
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

/// Imported scene objects prepared from the supplied GLB assets.
#[derive(Default)]
pub struct ImportedModels {
    pub solar_ship: Option<Mesh>,
    pub wormhole_ship: Option<Mesh>,
    pub gargantua: Option<Mesh>,
    pub accretion_disc: Option<Mesh>,
    pub endurance: Option<Mesh>,
    pub asteroid: Option<Mesh>,
}

pub struct Renderer {
    width: usize,
    height: usize,
    sphere: Mesh,
    solar_ship: Mesh,
    wormhole_ship: Mesh,
    gargantua_model: Option<Mesh>,
    accretion_disc_model: Option<Mesh>,
    endurance_model: Option<Mesh>,
    asteroid_model: Mesh,
    cube: Mesh,
    rock: Mesh,
    torus: Mesh,
    ring: Mesh,
    stars: Vec<Star>,
    asteroids: Vec<(Vec3, Vec3, f32)>,
}

impl Renderer {
    pub fn new(width: usize, height: usize, imported_sphere: Option<Mesh>, models: ImportedModels) -> Self {
        let sphere = imported_sphere.unwrap_or_else(|| Mesh::uv_sphere(18, 28));
        let stars = make_stars(1500);
        let asteroids = make_asteroids();
        Self {
            width,
            height,
            sphere,
            solar_ship: models.solar_ship.unwrap_or_else(Mesh::spaceship),
            wormhole_ship: models.wormhole_ship.unwrap_or_else(Mesh::spaceship),
            gargantua_model: models.gargantua,
            accretion_disc_model: models.accretion_disc,
            endurance_model: models.endurance,
            asteroid_model: models.asteroid.unwrap_or_else(Mesh::octahedron),
            cube: Mesh::cube(),
            rock: Mesh::octahedron(),
            torus: Mesh::torus(1.0, 0.12, 64, 10),
            ring: Mesh::ring(1.0, 2.5, 96),
            stars,
            asteroids,
        }
    }

    pub fn render(&self, framebuffer: &mut Framebuffer, game: &Game) {
        self.render_with_layer(framebuffer, game, 1);
    }

    /// Render through the selected shader-layer slot. Layer 1 is implemented;
    /// layers 2-6 are intentionally accepted and currently route through the
    /// same base pipeline so future shader layers are CLI/UI compatible.
    pub fn render_with_layer(&self, framebuffer: &mut Framebuffer, game: &Game, shader_layer: u8) {
        let _selected_layer = shader_layer.clamp(1, 6);
        framebuffer.clear_vertical_gradient(0x02040B, 0x080D1A);
        let (eye, target) = game.camera();
        let view = Mat4::look_at(eye, target, game.camera_up());
        let projection = Mat4::perspective(
            62.0_f32.to_radians(),
            self.width as f32 / self.height as f32,
            0.1,
            3_000_000.0,
        );
        let view_projection = projection * view;

        self.render_stars(framebuffer, eye, view_projection);
        self.render_space_bodies(framebuffer, game, eye, view_projection);
        if !game.destroyed && !game.is_overview() {
            self.render_ship(framebuffer, game, eye, view_projection);
        }
        if !game.is_overview() {
            self.render_speed_effect(framebuffer, game);
        }

        // Apply lensing after the opaque scene and ship have been drawn. The
        // existing Z-buffer protects anything closer than Gargantua.
        if let Some(gargantua) = game.bodies.iter().find(|body| {
            body.kind == PlanetKind::Gargantua && game.body_is_visible(body)
        }) {
            let cull_distance = if game.is_overview() { 2_900_000.0 } else { 1_500_000.0 };
            if distance(gargantua.position, eye) <= cull_distance {
                self.apply_gargantua_lensing(framebuffer, gargantua.position, view, view_projection);
                            // Segunda pasada: dibuja solo la mitad frontal del disco sin distorsión
                // para conservar la línea horizontal delante del horizonte negro.
                if let (Some(model), Some((_, _, bh_depth))) = (
                    &self.accretion_disc_model,
                    project_screen(gargantua.position, view_projection, self.width, self.height),
                ) {
                    let layer_scale = GARGANTUA_DISK_RADIUS / 2.6;
                    let disk_base = Mat4::translation(gargantua.position)
                        * Mat4::rotation_y(game.clock * 0.006)
                        * Mat4::rotation_x(-std::f32::consts::FRAC_PI_2)
                        * Mat4::scale(vec3(layer_scale, layer_scale, layer_scale));
                    self.draw_accretion_disc(framebuffer, model, disk_base, view_projection, 0.85, bh_depth);
                }
            
            }
        }
    }

    /// Screen-space radial streaks inspired by the supplied hyperspace reference.
    /// Regular Shift boost is intentionally faint; Shift+B produces the full warp tunnel.
    fn render_speed_effect(&self, fb: &mut Framebuffer, game: &Game) {
        if !game.boost_active {
            return;
        }

        let superboost = game.superboost_active;
        let ray_count = if superboost { 220 } else { 18 };
        let center_x = self.width as f32 * 0.5;
        let center_y = self.height as f32 * 0.5;
        let max_radius = (center_x * center_x + center_y * center_y).sqrt() * 1.08;
        let golden_angle = 2.399_963_1_f32;

        for index in 0..ray_count {
            let seed = index as f32 * 91.731 + 17.17;
            let random_a = (seed.sin() * 43_758.547).fract().abs();
            let random_b = ((seed + 31.416).sin() * 24_634.635).fract().abs();
            let angle = index as f32 * golden_angle
                + game.clock * if superboost { 0.055 } else { 0.018 }
                + (random_a - 0.5) * 0.22;
            let (sin_angle, cos_angle) = angle.sin_cos();

            // Leave a dark breathing space around the vanishing point, as in
            // the reference image, so the ship/reticle remain readable.
            let inner_radius = if superboost { 26.0 + random_a * 66.0 } else { 42.0 + random_a * 52.0 };
            let outer_radius = max_radius * if superboost {
                0.30 + random_b * 0.82
            } else {
                0.24 + random_b * 0.52
            };
            let start_x = center_x + cos_angle * inner_radius;
            let start_y = center_y + sin_angle * inner_radius;
            let end_x = center_x + cos_angle * outer_radius;
            let end_y = center_y + sin_angle * outer_radius;

            let color = if index % 7 == 0 {
                0x79BFFF
            } else if index % 5 == 0 {
                0xD9F0FF
            } else {
                0xFFFFFF
            };
            let alpha = if superboost {
                (82.0 + random_a * 135.0) as u8
            } else {
                (7.0 + random_a * 15.0) as u8
            };
            fb.draw_line_blended(
                start_x.round() as i32,
                start_y.round() as i32,
                end_x.round() as i32,
                end_y.round() as i32,
                color,
                alpha,
            );

            // A few streaks get a very thin luminous core, but avoid thick neon bars.
            if superboost && index % 9 == 0 {
                fb.draw_line_blended(
                    start_x.round() as i32 + 1,
                    start_y.round() as i32,
                    end_x.round() as i32 + 1,
                    end_y.round() as i32,
                    0xB8DCFF,
                    (alpha / 2).max(24),
                );
            }
        }
    }

    fn render_stars(&self, framebuffer: &mut Framebuffer, eye: Vec3, view_projection: Mat4) {
        for star in &self.stars {
            // Place each star far from the camera along a fixed direction. This
            // keeps the background stable while avoiding an enormous world mesh.
            let position = eye + star.direction * 1_300_000.0;
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
        // Orbital paths are drawn first so planet meshes naturally cover the
        // parts of the lines that pass behind their surfaces in the Z-buffer.
        if !game.crossed_wormhole {
            self.render_solar_orbits(framebuffer, game, vp);
        } else {
            self.render_gargantua_orbits(framebuffer, game, vp);
        }

        let body_cull_distance = if game.is_overview() { 2_900_000.0 } else { 1_500_000.0 };
        for body in &game.bodies {
            if !game.body_is_visible(body) {
                continue;
            }
            if distance(body.position, eye) > body_cull_distance {
                continue;
            }
            self.render_body(framebuffer, body, game.clock, eye, vp);
        }

        if !game.crossed_wormhole {
            let belt_marker = game.bodies.iter().find(|body| body.kind == PlanetKind::AsteroidBelt);
            let sun = game.bodies.iter().find(|body| body.kind == PlanetKind::Sun);
            if let (Some(marker), Some(sun)) = (belt_marker, sun) {
                // Asteroids are distributed across the complete 360-degree belt.
                // The belt marker is retained only for navigation and visibility.
                let belt_cull_distance = if game.is_overview() { 2_900_000.0 } else { 400_000.0 };
                if distance(marker.position, eye) < belt_cull_distance {
                    self.render_asteroid_belt(framebuffer, sun.position, eye, vp, game.clock, belt_cull_distance);
                }
            }
        }
    }

    /// Draw circular orbital guides around the Sun. Planet radii use compressed
    /// AU units but preserve each planet's approximate semi-major-axis ratio.
    fn render_solar_orbits(&self, fb: &mut Framebuffer, game: &Game, vp: Mat4) {
        let Some(sun) = game.bodies.iter().find(|body| body.kind == PlanetKind::Sun) else {
            return;
        };

        for (kind, color) in [
            (PlanetKind::Mercury, 0x8B7A65),
            (PlanetKind::Venus, 0xB58B55),
            (PlanetKind::Earth, 0x285A78),
            (PlanetKind::Mars, 0x765344),
            (PlanetKind::Jupiter, 0x78634A),
            (PlanetKind::Saturn, 0x756C50),
            (PlanetKind::Uranus, 0x4F8790),
            (PlanetKind::Neptune, 0x3D5F9B),
        ] {
            if let Some(planet) = game.bodies.iter().find(|body| body.kind == kind) {
                let offset = planet.position - sun.position;
                let radius = (offset.x * offset.x + offset.z * offset.z).sqrt();
                let axis_u = normalize(vec3(offset.x, 0.0, offset.z));
                let mut axis_v = normalize(cross(vec3(0.0, 1.0, 0.0), axis_u));
                if length(axis_v) < 0.001 {
                    axis_v = normalize(cross(vec3(1.0, 0.0, 0.0), axis_u));
                }
                self.draw_orbit_line(fb, sun.position, axis_u, axis_v, radius, color, vp, 180);
            }
        }

        // The asteroid field is shown as individual rocks, without circular
        // guide lines that could make the belt look like a solid ring again.

        // Earth's Moon is the only moon currently represented as a world in
        // this journey, so its guide is centered on Earth rather than the Sun.
        if let (Some(earth), Some(moon)) = (
            game.bodies.iter().find(|body| body.kind == PlanetKind::Earth),
            game.bodies.iter().find(|body| body.kind == PlanetKind::Moon),
        ) {
            let offset = moon.position - earth.position;
            let radius = (offset.x * offset.x + offset.z * offset.z).sqrt();
            let axis_u = normalize(vec3(offset.x, 0.0, offset.z));
            let mut axis_v = normalize(cross(vec3(0.0, 1.0, 0.0), axis_u));
            if length(axis_v) < 0.001 {
                axis_v = normalize(cross(vec3(1.0, 0.0, 0.0), axis_u));
            }
            self.draw_orbit_line(fb, earth.position, axis_u, axis_v, radius, 0x697987, vp, 96);
        }
    }

    /// Draw the three known planets around Gargantua in the second-system view.
    fn render_gargantua_orbits(&self, fb: &mut Framebuffer, game: &Game, vp: Mat4) {
        let Some(center_body) = game.bodies.iter().find(|body| body.kind == PlanetKind::Gargantua) else {
            return;
        };
        for (kind, color) in [
            (PlanetKind::MillerWater, 0x28677C),
            (PlanetKind::MannIce, 0x71899A),
            (PlanetKind::EdmundHabitable, 0x526A4B),
        ] {
            if let Some(planet) = game.bodies.iter().find(|body| body.kind == kind) {
                let offset = planet.position - center_body.position;
                let radius = (offset.x * offset.x + offset.z * offset.z).sqrt();
                let axis_u = normalize(vec3(offset.x, 0.0, offset.z));
                let mut axis_v = normalize(cross(vec3(0.0, 1.0, 0.0), axis_u));
                if length(axis_v) < 0.001 {
                    axis_v = normalize(cross(vec3(1.0, 0.0, 0.0), axis_u));
                }
                self.draw_orbit_line(fb, center_body.position, axis_u, axis_v, radius, color, vp, 150);
            }
        }
    }

    fn draw_orbit_line(
        &self,
        fb: &mut Framebuffer,
        center: Vec3,
        axis_u: Vec3,
        axis_v: Vec3,
        radius: f32,
        color: u32,
        vp: Mat4,
        segments: usize,
    ) {
        if radius <= 1.0 || segments < 3 {
            return;
        }
        let mut previous: Option<(i32, i32, f32)> = None;
        for index in 0..=segments {
            let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
            let point = center + axis_u * (radius * angle.cos()) + axis_v * (radius * angle.sin());
            let current = project_point(point, vp, self.width, self.height);
            if let (Some((x0, y0, _)), Some((x1, y1, _))) = (previous, current) {
                fb.draw_line(x0, y0, x1, y1, color);
            }
            previous = current;
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
            }
            PlanetKind::Sun => {
                let model = Mat4::translation(position) * Mat4::scale(vec3(radius, radius, radius));
                self.draw_mesh(fb, &self.sphere, model, Material::new(vec3(1.0, 0.68, 0.25), ShaderKind::Sun).emissive(1.8), eye, vp, time);
            }
            PlanetKind::Mercury | PlanetKind::Venus | PlanetKind::Moon | PlanetKind::Mars |
            PlanetKind::Jupiter | PlanetKind::Uranus | PlanetKind::Neptune |
            PlanetKind::MillerWater | PlanetKind::MannIce | PlanetKind::EdmundHabitable => {
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
                // The anomaly is kept as one luminous spherical body; no outer rings.
                let model = Mat4::translation(position)
                    * Mat4::rotation_y(time * 0.08)
                    * Mat4::scale(vec3(radius, radius, radius));
                self.draw_mesh(fb, &self.sphere, model,
                    Material::new(vec3(0.12, 0.36, 0.62), ShaderKind::Wormhole).emissive(0.48),
                    eye, vp, time);
            }
            PlanetKind::Docking => {
                if let Some(endurance) = &self.endurance_model {
                    // This imported Endurance + Ranger model replaces the old docking torus.
                    let model = Mat4::translation(position)
                        * Mat4::rotation_y(time * 0.035 + std::f32::consts::FRAC_PI_2)
                        * Mat4::scale(vec3(radius * 1.4, radius * 1.4, radius * 1.4));
                    self.draw_mesh(fb, endurance, model,
                        Material::new(vec3(0.66, 0.70, 0.76), ShaderKind::Solid).specular(0.82),
                        eye, vp, time);
                } else {
                    let model = Mat4::translation(position) * Mat4::rotation_z(time * 0.08) * Mat4::scale(vec3(radius * 1.4, radius * 1.4, radius * 0.3));
                    self.draw_mesh(fb, &self.torus, model, Material::new(vec3(0.62, 0.72, 0.78), ShaderKind::Solid).specular(0.75), eye, vp, time);
                }
            }
            PlanetKind::Gargantua => {
                // The supplied 22-layer golden gas OBJ has a 5.2-unit width.
                // Rotate Blender's Z-up disk into this renderer's Y-up world.
                // Its outer edge remains 1,600 world units inside Miller's orbit.
                if let Some(model) = &self.accretion_disc_model {
                    let layer_scale = GARGANTUA_DISK_RADIUS / 2.6;
                    let disk_spin = Mat4::rotation_y(time * 0.006);
                    let disk_scale = Mat4::scale(vec3(layer_scale, layer_scale, layer_scale));
                    let disk_base = Mat4::translation(position)
                        * disk_spin
                        * Mat4::rotation_x(-std::f32::consts::FRAC_PI_2)
                        * disk_scale;
                    self.draw_accretion_disc(fb, model, disk_base, vp, 1.0, 1.0);

                    // The asset is made from very thin, parallel planes. A pair
                    // of faint, slightly crossed copies keeps the gas visible
                    // from grazing camera angles without turning it into a
                    // thick spherical shell. Their low contribution preserves
                    // the original 22-layer disc as the dominant image.
                    let cross_tilt = 12.0_f32.to_radians();
                    let disk_cross_x = Mat4::translation(position)
                        * disk_spin
                        * Mat4::rotation_z(cross_tilt)
                        * Mat4::rotation_x(-std::f32::consts::FRAC_PI_2)
                        * disk_scale;
                    let disk_cross_z = Mat4::translation(position)
                        * disk_spin
                        * Mat4::rotation_x(cross_tilt)
                        * Mat4::rotation_x(-std::f32::consts::FRAC_PI_2)
                        * disk_scale;
                    self.draw_accretion_disc(fb, model, disk_cross_x, vp, 0.22, 1.0);
                    self.draw_accretion_disc(fb, model, disk_cross_z, vp, 0.22, 1.0);
                } else if let Some(model) = &self.gargantua_model {
                    // Compatibility fallback for projects where the golden
                    // 22-layer asset is absent.
                    let black_hole = Mat4::translation(position)
                        * Mat4::rotation_y(time * 0.006)
                        * Mat4::scale(vec3(GARGANTUA_DISK_RADIUS, GARGANTUA_DISK_RADIUS, GARGANTUA_DISK_RADIUS));
                    self.draw_mesh(fb, model, black_hole,
                        Material::new(vec3(0.18, 0.13, 0.22), ShaderKind::Solid).specular(0.42),
                        eye, vp, time);
                } else {
                    let black_hole = Mat4::translation(position)
                        * Mat4::scale(vec3(GARGANTUA_RADIUS, GARGANTUA_RADIUS, GARGANTUA_RADIUS));
                    self.draw_mesh(fb, &self.sphere, black_hole,
                        Material::new(vec3(0.004, 0.003, 0.008), ShaderKind::Solid), eye, vp, time);
                }
            }
            PlanetKind::AsteroidBelt => {}
        }
    }

    /// Draw one 22-plane transparent gas stack. The base stack plus two faint
    /// crossed stacks make the accretion ring readable from grazing angles.
    /// Each pass is additive and depth-tested but never writes the Z-buffer.
    fn draw_accretion_disc(
        &self,
        fb: &mut Framebuffer,
        mesh: &Mesh,
        model: Mat4,
        vp: Mat4,
        opacity_scale: f32,
        max_depth: f32,
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
            if clip[3] <= 0.1 || !clip[3].is_finite() {
                transformed.push(None);
                continue;
            }
            let inv_w = 1.0 / clip[3];
            let ndc_x = clip[0] * inv_w;
            let ndc_y = clip[1] * inv_w;
            let ndc_z = clip[2] * inv_w;
            if !ndc_x.is_finite() || !ndc_y.is_finite() || !ndc_z.is_finite() {
                transformed.push(None);
                continue;
            }
            transformed.push(Some(ScreenVertex {
                x: (ndc_x * 0.5 + 0.5) * self.width as f32,
                y: (0.5 - ndc_y * 0.5) * self.height as f32,
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
            // Every OBJ layer is a flat square at one local Z coordinate.
            let layer_z = (mesh.vertices[triangle[0]].position.z
                + mesh.vertices[triangle[1]].position.z
                + mesh.vertices[triangle[2]].position.z) / 3.0;
            let (texture, opacity) = disc_layer_texture(layer_z);
            self.rasterize_additive_textured_triangle(fb, a, b, c, texture, opacity * opacity_scale, max_depth);
        }
    }

    fn rasterize_additive_textured_triangle(
        &self,
        fb: &mut Framebuffer,
        a: ScreenVertex,
        b: ScreenVertex,
        c: ScreenVertex,
        texture: DiscTexture,
        opacity: f32,
        max_depth: f32,
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
                if !(0.0..=1.0).contains(&depth) || depth > max_depth {
                    continue;
                }
                let index = y as usize * self.width + x as usize;
                // Smaller depth means closer. Keep foreground ships/planets
                // intact and leave the existing depth value untouched.
                if depth >= fb.depth[index] {
                    continue;
                }

                let reciprocal_w = w0 * a.inv_w + w1 * b.inv_w + w2 * c.inv_w;
                if reciprocal_w.abs() < 1.0e-8 {
                    continue;
                }
                let correction = 1.0 / reciprocal_w;
                let uv = [
                    (a.uv_over_w[0] * w0 + b.uv_over_w[0] * w1 + c.uv_over_w[0] * w2) * correction,
                    (a.uv_over_w[1] * w0 + b.uv_over_w[1] * w1 + c.uv_over_w[1] * w2) * correction,
                ];
                let texel = sample_disc_texture(texture, uv);
                let amount = (texel[3] * opacity * 5.0).clamp(0.0, 1.0);
                if amount <= 0.0005 {
                    continue;
                }
                let base = fb.color[index];
                let r = linear_to_srgb((srgb_to_linear(((base >> 16) & 0xFF) as f32 / 255.0)
                    + texel[0] * amount).clamp(0.0, 1.0));
                let g = linear_to_srgb((srgb_to_linear(((base >> 8) & 0xFF) as f32 / 255.0)
                    + texel[1] * amount).clamp(0.0, 1.0));
                let blue = linear_to_srgb((srgb_to_linear((base & 0xFF) as f32 / 255.0)
                    + texel[2] * amount).clamp(0.0, 1.0));
                fb.color[index] = (((r * 255.0).round() as u32) << 16)
                    | (((g * 255.0).round() as u32) << 8)
                    | (blue * 255.0).round() as u32;
            }
        }
    }

    /// Radial post-process around the projected event horizon. Work is limited
    /// to the black hole's screen-space bounding box. The Z-buffer preserves
    /// foreground geometry, while the exclusion mask prevents opaque meshes
    /// from being picked up and duplicated by bilinear source sampling.
    fn apply_gargantua_lensing(&self, fb: &mut Framebuffer, world_pos: Vec3, view: Mat4, vp: Mat4) {
        if self.width < 2 || self.height < 2 {
            return;
        }
        let Some((cx, cy, bh_depth)) = project_screen(world_pos, vp, self.width, self.height) else {
            return;
        };
        let camera_right = vec3(view.m[0][0], view.m[0][1], view.m[0][2]);
        let horizon_edge = world_pos + camera_right * GARGANTUA_RADIUS;
        let Some((edge_x, edge_y, _)) = project_screen(horizon_edge, vp, self.width, self.height) else {
            return;
        };
        let r_s = ((edge_x - cx).powi(2) + (edge_y - cy).powi(2)).sqrt().max(2.0);
        let r_max = (r_s * 4.5).max(r_s + 2.0);
        let width = fb.width;
        let height = fb.height;
        fb.postprocess_source.copy_from_slice(&fb.color);
        apply_black_hole_lensing(
            &fb.postprocess_source,
            &mut fb.color,
            &fb.depth,
            &fb.lensing_excluded,
            width,
            height,
            cx,
            cy,
            bh_depth,
            r_s,
            r_max,
        );
    }

    fn render_asteroid_belt(
        &self, fb: &mut Framebuffer, sun: Vec3, eye: Vec3, vp: Mat4, time: f32, max_distance: f32,
    ) {
        // Each rock follows its own Keplerian speed: inner asteroids orbit
        // faster than outer ones instead of rotating as a rigid disc.
        for (offset, size, rotation) in &self.asteroids {
            let orbital_radius_au = (offset.x * offset.x + offset.z * offset.z).sqrt() / AU_SCALE;
            let orbital_period = 300.0 * orbital_radius_au.max(0.1).powf(1.5);
            let angle = time * (std::f32::consts::TAU / orbital_period);
            let (sin_angle, cos_angle) = angle.sin_cos();
            let rotated = vec3(
                offset.x * cos_angle + offset.z * sin_angle,
                offset.y,
                -offset.x * sin_angle + offset.z * cos_angle,
            );
            let position = sun + rotated;
            if distance(position, eye) > max_distance {
                continue;
            }
            let model = Mat4::translation(position)
                * Mat4::rotation_y(*rotation + time * 0.025)
                * Mat4::rotation_x(*rotation * 0.7 + time * 0.013)
                * Mat4::scale(*size);
            self.draw_mesh(fb, &self.asteroid_model, model,
                Material::new(vec3(0.37, 0.34, 0.31), ShaderKind::Rock), eye, vp, time);
        }
    }

    fn render_ship(&self, fb: &mut Framebuffer, game: &Game, eye: Vec3, vp: Mat4) {
        // These imported GLBs use different local axes. Endurance's ring lies
        // in the YZ plane, so it needs a quarter-turn to face the chase camera.
        // Ranger's aft engine is on its -Z end, so a half-turn brings that rear
        // face toward the camera. Only the mesh orientation changes; flight
        // direction, yaw, pitch, and thrust remain in the game's own basis.
        let (ship_mesh, ship_scale, local_yaw) = if game.crossed_wormhole {
            (&self.wormhole_ship, 4.8, std::f32::consts::PI)
        } else {
            (&self.solar_ship, 5.0, std::f32::consts::FRAC_PI_2)
        };
        let model = Mat4::translation(game.position)
            * Mat4::rotation_y(game.yaw)
            * Mat4::rotation_x(game.pitch)
            * Mat4::rotation_y(local_yaw)
            * Mat4::scale(vec3(ship_scale, ship_scale, ship_scale));
        self.draw_mesh(fb, ship_mesh, model,
            Material::new(vec3(0.72, 0.79, 0.83), ShaderKind::Solid).specular(0.82),
            eye, vp, game.clock);

        // Engine plumes stay restrained during normal boost; superboost gets
        // a longer and brighter exhaust to reinforce the stronger warp tunnel.
        let right = game.right();
        let up = game.up();
        let backward = -game.forward();
        let exhaust_length = if game.superboost_active {
            2.6
        } else if game.boost_active {
            0.68
        } else if game.speed > 180.0 {
            0.85
        } else {
            0.55
        };
        let exhaust_glow = if game.superboost_active { 1.35 } else if game.boost_active { 0.32 } else { 0.55 };
        for side in [-0.42, 0.42] {
            let center = game.position + right * side + backward * 3.15;
            let exhaust_model = Mat4::translation(center)
                * Mat4::rotation_y(game.yaw)
                * Mat4::rotation_x(game.pitch)
                * Mat4::scale(vec3(0.12, 0.12, exhaust_length));
            let _ = up; // Kept explicit to show the ship's local basis.
            self.draw_mesh(fb, &self.torus, exhaust_model, Material::new(vec3(0.20, 0.78, 1.0), ShaderKind::Wormhole).emissive(exhaust_glow), eye, vp, game.clock);
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
                let index = y as usize * fb.width + x as usize;
                if depth < fb.depth[index] {
                    // Do not let planets, ships, asteroids or other opaque mesh
                    // fragments be copied into a second position by lensing.
                    // Their original pixels are still protected by the Z-test.
                    fb.lensing_excluded[index] = 1;
                }
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
        PlanetKind::Mercury => Material::new(vec3(0.48, 0.46, 0.43), ShaderKind::Mercury),
        PlanetKind::Venus => Material::new(vec3(0.86, 0.64, 0.34), ShaderKind::Venus),
        PlanetKind::Earth => Material::new(vec3(0.22, 0.48, 0.78), ShaderKind::Earth).specular(0.38),
        PlanetKind::Moon => Material::new(vec3(0.55, 0.57, 0.58), ShaderKind::Moon),
        PlanetKind::Mars => Material::new(vec3(0.72, 0.29, 0.14), ShaderKind::Mars),
        PlanetKind::Jupiter => Material::new(vec3(0.72, 0.48, 0.31), ShaderKind::Jupiter),
        PlanetKind::Saturn => Material::new(vec3(0.77, 0.66, 0.45), ShaderKind::Saturn),
        PlanetKind::Uranus => Material::new(vec3(0.40, 0.74, 0.77), ShaderKind::Uranus).specular(0.3),
        PlanetKind::Neptune => Material::new(vec3(0.16, 0.29, 0.76), ShaderKind::Neptune).specular(0.42),
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
        ShaderKind::Mercury => {
            let crater = noise3(normal * 31.0) + noise3(normal * 12.0) * 0.35;
            let shade = 0.68 + crater * 0.42;
            vec3(0.43, 0.42, 0.40) * shade
        }
        ShaderKind::Venus => {
            let cloud = (v * 52.0 + (u * 13.0).sin() * 3.2 + time * 0.025).sin();
            let turbulence = noise3(normal * 18.0) * 0.12;
            vec3(0.78 + cloud * 0.08 + turbulence,
                 0.57 + cloud * 0.07 + turbulence,
                 0.30 + cloud * 0.035)
        }
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
        ShaderKind::Uranus => {
            let band = (v * 25.0 + (u * 4.0).sin() * 0.3).sin() * 0.035;
            vec3(0.35 + band, 0.68 + band, 0.73 + band)
        }
        ShaderKind::Neptune => {
            let band = (v * 38.0 + (u * 8.0).sin() * 0.8).sin() * 0.07;
            let storm = ((u - 0.65).powi(2) * 2.0 + (v - 0.42).powi(2) * 5.0).sqrt();
            if storm < 0.10 { vec3(0.08, 0.15, 0.46) }
            else { vec3(0.10 + band, 0.24 + band, 0.67 + band) }
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

    }
}

fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (px - ax) * (by - ay) - (py - ay) * (bx - ax)
}

#[derive(Clone, Copy)]
struct DiscTexture {
    pixels: &'static [u8],
    width: usize,
    height: usize,
}

const DISC_CENTER_TEXTURE: DiscTexture = DiscTexture {
    pixels: include_bytes!("../assets/blackhole_dorado/disco_centro.rgba"),
    width: 1024,
    height: 1024,
};
const DISC_MIDDLE_TEXTURE: DiscTexture = DiscTexture {
    pixels: include_bytes!("../assets/blackhole_dorado/disco_medio.rgba"),
    width: 1024,
    height: 1024,
};
const DISC_EDGE_TEXTURE: DiscTexture = DiscTexture {
    pixels: include_bytes!("../assets/blackhole_dorado/disco_borde.rgba"),
    width: 1024,
    height: 1024,
};

fn disc_layer_texture(local_z: f32) -> (DiscTexture, f32) {
    let height = local_z.abs();
    if height >= 0.045 {
        (DISC_EDGE_TEXTURE, 0.036)
    } else if height >= 0.015 {
        (DISC_MIDDLE_TEXTURE, 0.057)
    } else {
        (DISC_CENTER_TEXTURE, 0.078)
    }
}

/// Bilinear RGBA lookup. RGB is converted from sRGB to linear light while
/// alpha stays linear, matching the supplied layer-texture documentation.
fn sample_disc_texture(texture: DiscTexture, uv: [f32; 2]) -> [f32; 4] {
    let fx = uv[0].clamp(0.0, 1.0) * (texture.width - 1) as f32;
    let fy = uv[1].clamp(0.0, 1.0) * (texture.height - 1) as f32;
    let x0 = fx.floor() as usize;
    let y0 = fy.floor() as usize;
    let x1 = (x0 + 1).min(texture.width - 1);
    let y1 = (y0 + 1).min(texture.height - 1);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let sample = |x: usize, y: usize| -> [f32; 4] {
        let offset = (y * texture.width + x) * 4;
        [
            texture.pixels[offset] as f32 / 255.0,
            texture.pixels[offset + 1] as f32 / 255.0,
            texture.pixels[offset + 2] as f32 / 255.0,
            texture.pixels[offset + 3] as f32 / 255.0,
        ]
    };
    let c00 = sample(x0, y0);
    let c10 = sample(x1, y0);
    let c01 = sample(x0, y1);
    let c11 = sample(x1, y1);
    let mut result = [0.0; 4];
    for channel in 0..4 {
        let top = c00[channel] * (1.0 - tx) + c10[channel] * tx;
        let bottom = c01[channel] * (1.0 - tx) + c11[channel] * tx;
        result[channel] = top * (1.0 - ty) + bottom * ty;
    }
    result[0] = srgb_to_linear(result[0]);
    result[1] = srgb_to_linear(result[1]);
    result[2] = srgb_to_linear(result[2]);
    result
}

#[inline]
fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
}

#[inline]
fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 { value * 12.92 } else { 1.055 * value.powf(1.0 / 2.4) - 0.055 }
}

fn project_screen(point: Vec3, vp: Mat4, width: usize, height: usize) -> Option<(f32, f32, f32)> {
    let clip = vp.transform_vec4([point.x, point.y, point.z, 1.0]);
    if clip[3] <= 0.1 || !clip[3].is_finite() {
        return None;
    }
    let inv_w = 1.0 / clip[3];
    let ndc_x = clip[0] * inv_w;
    let ndc_y = clip[1] * inv_w;
    let depth = clip[2] * inv_w;
    if !ndc_x.is_finite() || !ndc_y.is_finite() || !(0.0..=1.0).contains(&depth) {
        return None;
    }
    Some((
        (ndc_x * 0.5 + 0.5) * width as f32,
        (0.5 - ndc_y * 0.5) * height as f32,
        depth,
    ))
}

fn apply_black_hole_lensing(
    src_buffer: &[u32],
    dst_buffer: &mut [u32],
    z_buffer: &[f32],
    exclusion_mask: &[u8],
    width: usize,
    height: usize,
    cx: f32,
    cy: f32,
    bh_depth: f32,
    r_s: f32,
    r_max: f32,
) {
    if src_buffer.len() != width * height
        || dst_buffer.len() != width * height
        || z_buffer.len() != width * height
        || exclusion_mask.len() != width * height
        || width < 2
        || height < 2
    {
        return;
    }
    dst_buffer.copy_from_slice(src_buffer);
    let min_x = ((cx - r_max).floor() as isize).max(0) as usize;
    let max_x = ((cx + r_max).ceil() as isize).min(width as isize - 1);
    let min_y = ((cy - r_max).floor() as isize).max(0) as usize;
    let max_y = ((cy + r_max).ceil() as isize).min(height as isize - 1);
    if max_x < min_x as isize || max_y < min_y as isize {
        return;
    }
    let r_s_sq = r_s * r_s;
    let r_max_sq = r_max * r_max;
    for y in min_y..=max_y as usize {
        let dy = y as f32 - cy;
        let dy_sq = dy * dy;
        let row = y * width;
        for x in min_x..=max_x as usize {
            let dx = x as f32 - cx;
            let dist_sq = dx * dx + dy_sq;
            if dist_sq >= r_max_sq {
                continue;
            }
            let index = row + x;
            if z_buffer[index] < bh_depth {
                continue;
            }
            if dist_sq <= r_s_sq {
                dst_buffer[index] = 0x000000;
                continue;
            }
            let dist = dist_sq.sqrt();
            let falloff = (1.0 - (dist - r_s) / (r_max - r_s)).clamp(0.0, 1.0);
            let deflection = (r_s_sq / dist) * 2.15 * falloff;
            let inv_dist = 1.0 / dist;
            let sample_x = x as f32 - dx * inv_dist * deflection;
            let sample_y = y as f32 - dy * inv_dist * deflection;
            let mut color = sample_frame_bilinear_masked(
                src_buffer,
                exclusion_mask,
                width,
                height,
                sample_x,
                sample_y,
                index,
            );
            let edge_proximity = 1.0 - ((dist - r_s) / (r_s * 0.25)).clamp(0.0, 1.0);
            if edge_proximity > 0.0 {
                color = add_colors(color, 0xFFF0C2, edge_proximity * edge_proximity * 0.8);
            }
            dst_buffer[index] = color;
        }
    }
}

#[inline(always)]
fn sample_frame_bilinear_masked(
    buffer: &[u32],
    exclusion_mask: &[u8],
    width: usize,
    height: usize,
    fx: f32,
    fy: f32,
    fallback_index: usize,
) -> u32 {
    let x0 = (fx.floor() as isize).clamp(0, (width - 2) as isize) as usize;
    let y0 = (fy.floor() as isize).clamp(0, (height - 2) as isize) as usize;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let tx = (fx - x0 as f32).clamp(0.0, 1.0);
    let ty = (fy - y0 as f32).clamp(0.0, 1.0);
    let inv_tx = 1.0 - tx;
    let inv_ty = 1.0 - ty;
    let samples = [
        (y0 * width + x0, inv_tx * inv_ty),
        (y0 * width + x1, tx * inv_ty),
        (y1 * width + x0, inv_tx * ty),
        (y1 * width + x1, tx * ty),
    ];

    let mut weight_sum = 0.0_f32;
    let mut red = 0.0_f32;
    let mut green = 0.0_f32;
    let mut blue = 0.0_f32;
    for (index, weight) in samples {
        if weight <= 0.0 || exclusion_mask[index] != 0 {
            continue;
        }
        let color = buffer[index];
        red += ((color >> 16) & 0xFF) as f32 * weight;
        green += ((color >> 8) & 0xFF) as f32 * weight;
        blue += (color & 0xFF) as f32 * weight;
        weight_sum += weight;
    }

    if weight_sum > 1.0e-6 {
        let normalize = 1.0 / weight_sum;
        return (((red * normalize).round() as u32).min(255) << 16)
            | (((green * normalize).round() as u32).min(255) << 8)
            | (blue * normalize).round().min(255.0) as u32;
    }

    // A completely covered 2x2 sample should not reintroduce the object.
    // Pull the nearest unmasked background texel from a tiny neighborhood to
    // avoid a hard hole at the edge of a planet or ship silhouette.
    let center_x = fx.round() as isize;
    let center_y = fy.round() as isize;
    for radius in 1..=4_isize {
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                if ox.abs().max(oy.abs()) != radius {
                    continue;
                }
                let x = center_x + ox;
                let y = center_y + oy;
                if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
                    continue;
                }
                let index = y as usize * width + x as usize;
                if exclusion_mask[index] == 0 {
                    return buffer[index];
                }
            }
        }
    }

    if exclusion_mask[fallback_index] == 0 {
        buffer[fallback_index]
    } else {
        0x02040B
    }
}

#[inline(always)]
fn add_colors(base: u32, glow: u32, intensity: f32) -> u32 {
    let r = (((base >> 16) & 0xFF) as f32 + ((glow >> 16) & 0xFF) as f32 * intensity).min(255.0) as u32;
    let g = (((base >> 8) & 0xFF) as f32 + ((glow >> 8) & 0xFF) as f32 * intensity).min(255.0) as u32;
    let b = ((base & 0xFF) as f32 + (glow & 0xFF) as f32 * intensity).min(255.0) as u32;
    (r << 16) | (g << 8) | b
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

fn make_asteroids() -> Vec<(Vec3, Vec3, f32)> {
    let mut rng = SimpleRng::new(7331);
    let mut rocks = Vec::with_capacity(900);
    let inner = ASTEROID_BELT_INNER_AU * AU_SCALE;
    let outer = ASTEROID_BELT_OUTER_AU * AU_SCALE;
    let golden_angle = 2.399_963_1_f32;

    // Even angular spacing with a little deterministic jitter avoids the
    // accidental clumps and empty sectors of a small random sample.
    for index in 0..900 {
        let angle = index as f32 * golden_angle + rng.range(-0.045, 0.045);
        // Uniform density by annular area rather than by radius.
        let radius = (inner * inner + rng.next() * (outer * outer - inner * inner)).sqrt();
        let y = rng.range(-180.0, 180.0);
        let position = vec3(angle.cos() * radius, y, angle.sin() * radius);
        // Most rocks remain small, with a few medium and very large bodies
        // breaking up the silhouette of the belt.
        let size = if index % 83 == 0 {
            rng.range(68.0, 110.0)
        } else if index % 13 == 0 {
            rng.range(34.0, 58.0)
        } else {
            rng.range(8.0, 24.0)
        };
        let shape = vec3(
            size * rng.range(0.72, 1.28),
            size * rng.range(0.58, 1.08),
            size * rng.range(0.70, 1.35),
        );
        let rotation = rng.range(0.0, std::f32::consts::TAU);
        rocks.push((position, shape, rotation));
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


#[cfg(test)]
mod asteroid_tests {
    use super::*;

    #[test]
    fn gargantua_disk_stops_1600_units_before_millers_orbit() {
        use crate::game::{GARGANTUA_DISK_ORBIT_GAP, GARGANTUA_DISK_RADIUS, GARGANTUA_MILLER_ORBIT};
        assert!((GARGANTUA_MILLER_ORBIT - GARGANTUA_DISK_RADIUS - GARGANTUA_DISK_ORBIT_GAP).abs() < 0.001);
    }

    #[test]
    fn lensing_blacks_out_the_horizon_but_preserves_foreground_geometry() {
        let width = 9;
        let height = 9;
        let src = vec![0x123456; width * height];
        let mut dst = vec![0; width * height];
        let mut depth = vec![f32::INFINITY; width * height];
        depth[4 * width + 4] = 0.1; // Foreground object at the projected center.

        let exclusion = vec![0; width * height];
        apply_black_hole_lensing(
            &src, &mut dst, &depth, &exclusion, width, height, 4.0, 4.0, 0.5, 1.5, 5.0,
        );

        assert_eq!(dst[4 * width + 4], 0x123456);
        assert_eq!(dst[4 * width + 5], 0x000000);
        assert_eq!(dst[0], 0x123456);
    }

    #[test]
    fn lensing_never_samples_opaque_objects_into_a_second_position() {
        let width = 9;
        let height = 9;
        let mut src = vec![0x112233; width * height];
        let mut dst = vec![0; width * height];
        let depth = vec![f32::INFINITY; width * height];
        let mut exclusion = vec![0; width * height];
        // A red planet/ship fragment sits at the remapped source coordinate.
        let object_index = 4 * width + 5;
        src[object_index] = 0xFF0000;
        exclusion[object_index] = 1;

        apply_black_hole_lensing(
            &src, &mut dst, &depth, &exclusion, width, height, 4.0, 4.0, 0.5, 1.0, 5.0,
        );

        // Destination (6,4) is outside the horizon and maps close to (5,4).
        // It must receive only the unmasked background, never the red object.
        assert_ne!(dst[4 * width + 6], 0xFF0000);
        assert_eq!(dst[4 * width + 6], 0x112233);
    }

    #[test]
    fn inner_asteroids_have_shorter_periods_than_outer_asteroids() {
        let period = |radius_au: f32| 300.0 * radius_au.powf(1.5);
        assert!(period(ASTEROID_BELT_INNER_AU) < period(ASTEROID_BELT_OUTER_AU));
    }

    #[test]
    fn asteroid_field_covers_the_full_orbital_annulus() {
        let asteroids = make_asteroids();
        assert_eq!(asteroids.len(), 900);
        let inner = ASTEROID_BELT_INNER_AU * AU_SCALE;
        let outer = ASTEROID_BELT_OUTER_AU * AU_SCALE;
        let mut angular_bins = [0usize; 36];

        for (offset, _, _) in asteroids {
            let radius = (offset.x * offset.x + offset.z * offset.z).sqrt();
            assert!(radius >= inner && radius <= outer);
            let angle = offset.z.atan2(offset.x).rem_euclid(std::f32::consts::TAU);
            let bin = ((angle / std::f32::consts::TAU) * angular_bins.len() as f32) as usize;
            angular_bins[bin.min(angular_bins.len() - 1)] += 1;
        }

        // Every ten-degree sector must contain rocks; the belt cannot collapse
        // into the short visible arc from the previous implementation.
        assert!(angular_bins.iter().all(|count| *count >= 10));
    }
}
