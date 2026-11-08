//! Journey data and flight simulation, separated from drawing code.

use crate::math::{clamp, distance, length, normalize, smoothstep, vec3, Vec3};

pub const GARGANTUA_RADIUS: f32 = 185.0;
pub fn gargantua_position() -> Vec3 { vec3(1180.0, -112.0, -12450.0) }
pub fn wormhole_exit() -> Vec3 { vec3(-150.0, 12.0, -10000.0) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetKind {
    Earth,
    Sun,
    Moon,
    Mars,
    AsteroidBelt,
    Jupiter,
    Saturn,
    Wormhole,
    Docking,
    MillerWater,
    MannIce,
    EdmundHabitable,
    Gargantua,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePhase {
    Launch,
    Cruise,
    Flyby,
    Transition,
    Arrival,
    Landed,
    Destroyed,
    Completed,
}

impl GamePhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Launch => "LAUNCH",
            Self::Cruise => "CRUISE",
            Self::Flyby => "FLYBY",
            Self::Transition => "WORMHOLE",
            Self::Arrival => "ARRIVAL",
            Self::Landed => "LANDED",
            Self::Destroyed => "DESTROYED",
            Self::Completed => "COMPLETED",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpaceBody {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: PlanetKind,
    pub position: Vec3,
    pub radius: f32,
    pub landable: bool,
    pub description: &'static str,
    pub reveal_distance: f32,
}

pub fn journey_bodies() -> Vec<SpaceBody> {
    vec![
        SpaceBody {
            id: "earth", label: "EARTH", kind: PlanetKind::Earth,
            position: vec3(-126.0, -8.0, -96.0), radius: 70.0, landable: true,
            description: "Launch corridor aligned. Leaving the last blue horizon behind.",
            reveal_distance: 1500.0,
        },
        SpaceBody {
            id: "moon", label: "MOON", kind: PlanetKind::Moon,
            position: vec3(-110.0, 58.0, -300.0), radius: 19.0, landable: true,
            description: "A small grey companion world, cratered and airless.",
            reveal_distance: 1200.0,
        },
        SpaceBody {
            id: "mars", label: "MARS", kind: PlanetKind::Mars,
            position: vec3(-560.0, 62.0, -1350.0), radius: 22.0, landable: true,
            description: "Dusty red terrain, a thin atmosphere, and polar ice haze.",
            reveal_distance: 1500.0,
        },
        SpaceBody {
            id: "asteroids", label: "ASTEROID BELT", kind: PlanetKind::AsteroidBelt,
            position: vec3(220.0, -24.0, -3150.0), radius: 190.0, landable: false,
            description: "Tumbling rock fragments cross the transfer route.",
            reveal_distance: 1900.0,
        },
        SpaceBody {
            id: "jupiter", label: "JUPITER", kind: PlanetKind::Jupiter,
            position: vec3(980.0, -110.0, -5600.0), radius: 126.0, landable: false,
            description: "An immense banded gas giant with a red storm system.",
            reveal_distance: 2200.0,
        },
        SpaceBody {
            id: "sun", label: "THE SUN", kind: PlanetKind::Sun,
            position: vec3(4200.0, 620.0, -6200.0), radius: 115.0, landable: false,
            description: "A distant solar landmark, not a reachable target.",
            reveal_distance: f32::INFINITY,
        },
        SpaceBody {
            id: "saturn", label: "SATURN", kind: PlanetKind::Saturn,
            position: vec3(-820.0, -72.0, -8200.0), radius: 106.0, landable: false,
            description: "Pale gold atmosphere and a dense field of icy rings.",
            reveal_distance: 2200.0,
        },
        SpaceBody {
            id: "wormhole", label: "SATURN WORMHOLE", kind: PlanetKind::Wormhole,
            position: vec3(220.0, 28.0, -9000.0), radius: 38.0, landable: false,
            description: "Spacetime curvature stable. The Saturn anomaly is open.",
            reveal_distance: 1500.0,
        },
        SpaceBody {
            id: "docking", label: "ENDURANCE DOCKING", kind: PlanetKind::Docking,
            position: vec3(-130.0, 12.0, -10100.0), radius: 20.0, landable: false,
            description: "Docking rotation matched. A new galaxy lies ahead.",
            reveal_distance: 1400.0,
        },
        SpaceBody {
            id: "gargantua", label: "GARGANTUA", kind: PlanetKind::Gargantua,
            position: gargantua_position(), radius: GARGANTUA_RADIUS, landable: false,
            description: "Event horizon, photon ring, and a violent accretion disk.",
            reveal_distance: 4300.0,
        },
        SpaceBody {
            id: "miller", label: "MILLER'S PLANET", kind: PlanetKind::MillerWater,
            position: vec3(-620.0, -58.0, -12100.0), radius: 44.0, landable: true,
            description: "A water world close enough to feel Gargantua's pull.",
            reveal_distance: 2100.0,
        },
        SpaceBody {
            id: "mann", label: "MANN'S PLANET", kind: PlanetKind::MannIce,
            position: vec3(-1220.0, -132.0, -14800.0), radius: 38.0, landable: true,
            description: "Frozen atmosphere, fractured ice crust, and a weak beacon.",
            reveal_distance: 2100.0,
        },
        SpaceBody {
            id: "edmund", label: "EDMUND'S PLANET", kind: PlanetKind::EdmundHabitable,
            position: vec3(780.0, 120.0, -17800.0), radius: 42.0, landable: true,
            description: "Habitable signatures detected. Journey endpoint acquired.",
            reveal_distance: 2100.0,
        },
    ]
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InputState {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub pitch_up: bool,
    pub pitch_down: bool,
    pub boost: bool,
    pub interact_pressed: bool,
    pub wormhole_pressed: bool,
    pub takeoff_pressed: bool,
    pub screenshot_pressed: bool,
}

#[derive(Clone, Debug)]
pub struct HudState {
    pub title: String,
    pub body: String,
    pub phase: String,
    pub speed: f32,
    pub distance: f32,
    pub target: String,
    pub prompt: String,
    pub warning: String,
    pub scan_progress: f32,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub bodies: Vec<SpaceBody>,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub phase: GamePhase,
    pub crossed_wormhole: bool,
    pub landed_body: Option<usize>,
    pub destroyed: bool,
    pub transition_timer: f32,
    pub clock: f32,
    pub capture_notice: f32,
}

impl Game {
    pub fn new() -> Self {
        Self {
            bodies: journey_bodies(),
            position: vec3(0.0, 9.0, 34.0),
            velocity: vec3(0.0, 0.0, -46.0),
            yaw: 0.0,
            pitch: -0.03,
            speed: 58.0,
            phase: GamePhase::Launch,
            crossed_wormhole: false,
            landed_body: None,
            destroyed: false,
            transition_timer: 0.0,
            clock: 0.0,
            capture_notice: 0.0,
        }
    }

    pub fn forward(&self) -> Vec3 {
        let cos_pitch = self.pitch.cos();
        normalize(vec3(
            -self.yaw.sin() * cos_pitch,
            self.pitch.sin(),
            -self.yaw.cos() * cos_pitch,
        ))
    }

    pub fn right(&self) -> Vec3 {
        normalize(vec3(self.yaw.cos(), 0.0, -self.yaw.sin()))
    }

    pub fn up(&self) -> Vec3 {
        normalize(crate::math::cross(self.right(), self.forward()))
    }

    pub fn update(&mut self, dt: f32, input: InputState) {
        let dt = dt.clamp(0.0, 0.05);
        self.clock += dt;
        self.capture_notice = (self.capture_notice - dt).max(0.0);

        if input.screenshot_pressed {
            self.capture_notice = 2.5;
        }

        if self.destroyed {
            self.phase = GamePhase::Destroyed;
            self.speed = 0.0;
            self.velocity = vec3(0.0, 0.0, 0.0);
            if input.takeoff_pressed {
                self.restart();
            }
            return;
        }

        if let Some(body_index) = self.landed_body {
            self.phase = GamePhase::Landed;
            if input.takeoff_pressed {
                let body = &self.bodies[body_index];
                let direction = normalize(self.position - body.position);
                self.position = body.position + direction * (body.radius + 95.0);
                self.velocity = direction * 72.0;
                self.speed = 72.0;
                self.landed_body = None;
                self.phase = GamePhase::Launch;
            }
            return;
        }

        if self.phase == GamePhase::Transition {
            self.transition_timer += dt;
            self.speed = damp(self.speed, 360.0, 3.0, dt);
            self.position += vec3(0.0, 0.0, -1.0) * (self.speed * dt);
            self.yaw += dt * 1.2;
            if self.transition_timer > 2.35 {
                self.position = wormhole_exit();
                self.velocity = vec3(16.0, 2.0, -92.0);
                self.speed = 92.0;
                self.transition_timer = 0.0;
                self.phase = GamePhase::Arrival;
                self.crossed_wormhole = true;
            }
            return;
        }

        let yaw_input = (input.left as i32 - input.right as i32) as f32 * 1.35;
        let pitch_input = (input.pitch_up as i32 - input.pitch_down as i32) as f32 * 1.05;
        self.yaw += yaw_input * dt;
        self.pitch = clamp(self.pitch + pitch_input * dt, -1.25, 1.25);

        let forward = self.forward();
        let right = self.right();
        let up = self.up();

        let target_speed = if input.back {
            (self.speed - 140.0 * dt).max(8.0)
        } else if input.forward {
            if input.boost { 390.0 } else { 215.0 }
        } else {
            (self.speed * 0.996).max(32.0)
        };
        self.speed = damp(
            self.speed,
            target_speed,
            if input.forward || input.back { 2.4 } else { 0.45 },
            dt,
        );

        let mut desired_velocity = forward * self.speed;
        desired_velocity += right * ((input.right as i32 - input.left as i32) as f32 * 7.0);
        desired_velocity += up * ((input.pitch_up as i32 - input.pitch_down as i32) as f32 * 5.0);
        let alpha = 1.0 - (-dt * 1.8).exp();
        self.velocity += (desired_velocity - self.velocity) * alpha;
        let maximum_velocity = if input.boost { 420.0 } else { 260.0 };
        let velocity_length = length(self.velocity);
        if velocity_length > maximum_velocity {
            self.velocity = normalize(self.velocity) * maximum_velocity;
        }
        self.position += self.velocity * dt;

        if self.crossed_wormhole {
            let to_black_hole = gargantua_position() - self.position;
            let center_distance = length(to_black_hole);
            let kill_radius = GARGANTUA_RADIUS * 1.08;
            let gravity_radius = GARGANTUA_RADIUS * 3.25;
            if center_distance < kill_radius {
                self.destroyed = true;
                self.phase = GamePhase::Destroyed;
                return;
            } else if center_distance < gravity_radius {
                let pull = 1.0 - smoothstep(kill_radius, gravity_radius, center_distance);
                self.velocity += normalize(to_black_hole) * (pull * 90.0 * dt);
                self.speed = self.speed.max(length(self.velocity));
            }
        }

        // Prevent a direct collision with the Sun, while allowing a close fly-by.
        if let Some(sun) = self.bodies.iter().find(|body| body.kind == PlanetKind::Sun) {
            let from_sun = self.position - sun.position;
            let minimum_distance = sun.radius + 900.0;
            if length(from_sun) < minimum_distance {
                let away = if length(from_sun) > 0.001 { normalize(from_sun) } else { vec3(0.0, 0.0, 1.0) };
                self.position = sun.position + away * minimum_distance;
                self.velocity += away * (80.0 * dt);
                self.speed = self.speed.min(118.0);
            }
        }

        if !self.crossed_wormhole {
            if let Some(wormhole) = self.bodies.iter().find(|body| body.kind == PlanetKind::Wormhole) {
                let surface_distance = (distance(wormhole.position, self.position) - wormhole.radius).max(0.0);
                if input.wormhole_pressed && surface_distance < 380.0 {
                    self.phase = GamePhase::Transition;
                    self.transition_timer = 0.0;
                    return;
                }
            }
        }

        let nearest_landing = self.nearest_landable();
        if input.interact_pressed {
            if let Some((body_index, surface_distance)) = nearest_landing {
                if surface_distance < 220.0 {
                    let body = &self.bodies[body_index];
                    let outward = normalize(self.position - body.position);
                    self.position = body.position + outward * (body.radius + 18.0);
                    self.speed = 0.0;
                    self.velocity = vec3(0.0, 0.0, 0.0);
                    self.landed_body = Some(body_index);
                    self.phase = GamePhase::Landed;
                    return;
                }
            }
        }

        self.phase = if self.nearest_visible_body().map(|(_, d)| d < 420.0).unwrap_or(false) {
            GamePhase::Flyby
        } else if self.crossed_wormhole && self.position.z > -10500.0 {
            GamePhase::Arrival
        } else if self.position.z > -120.0 && !self.crossed_wormhole {
            GamePhase::Launch
        } else if self.crossed_wormhole && self.position.z < -17500.0 {
            GamePhase::Completed
        } else {
            GamePhase::Cruise
        };
    }

    pub fn restart(&mut self) {
        self.position = vec3(0.0, 9.0, 34.0);
        self.velocity = vec3(0.0, 0.0, -46.0);
        self.yaw = 0.0;
        self.pitch = -0.03;
        self.speed = 58.0;
        self.phase = GamePhase::Launch;
        self.crossed_wormhole = false;
        self.landed_body = None;
        self.destroyed = false;
        self.transition_timer = 0.0;
    }

    pub fn body_is_visible(&self, body: &SpaceBody) -> bool {
        let in_solar_system = matches!(
            body.kind,
            PlanetKind::Earth | PlanetKind::Sun | PlanetKind::Moon | PlanetKind::Mars |
            PlanetKind::AsteroidBelt | PlanetKind::Jupiter | PlanetKind::Saturn | PlanetKind::Wormhole
        );
        let in_gargantua_system = matches!(
            body.kind,
            PlanetKind::Docking | PlanetKind::Gargantua | PlanetKind::MillerWater |
            PlanetKind::MannIce | PlanetKind::EdmundHabitable
        );
        if self.crossed_wormhole != in_gargantua_system {
            return false;
        }
        if body.kind == PlanetKind::Earth || body.kind == PlanetKind::Sun {
            return true;
        }
        if !in_solar_system && !in_gargantua_system {
            return false;
        }
        let surface_distance = (distance(body.position, self.position) - body.radius).max(0.0);
        surface_distance < body.reveal_distance
    }

    pub fn nearest_visible_body(&self) -> Option<(usize, f32)> {
        self.bodies
            .iter()
            .enumerate()
            .filter(|(_, body)| self.body_is_visible(body))
            .map(|(index, body)| (index, (distance(body.position, self.position) - body.radius).max(0.0)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    pub fn nearest_landable(&self) -> Option<(usize, f32)> {
        self.bodies
            .iter()
            .enumerate()
            .filter(|(_, body)| body.landable && self.body_is_visible(body))
            .map(|(index, body)| (index, (distance(body.position, self.position) - body.radius).max(0.0)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    pub fn camera(&self) -> (Vec3, Vec3) {
        if self.destroyed {
            let eye = self.position + vec3(18.0, 12.0, 26.0);
            return (eye, gargantua_position());
        }
        if let Some(body_index) = self.landed_body {
            let _body = &self.bodies[body_index];
            let eye = self.position + vec3(0.0, 7.5, 18.0);
            let target = self.position + vec3(0.0, 2.6, -16.0);
            return (eye, target);
        }
        let direction = if length(self.velocity) > 2.0 {
            normalize(self.velocity)
        } else {
            self.forward()
        };
        let camera_right = normalize(crate::math::cross(direction, vec3(0.0, 1.0, 0.0)));
        let eye = self.position - direction * 28.0 + vec3(0.0, 10.0, 0.0) + camera_right * 3.0;
        let target = self.position + direction * 22.0;
        (eye, target)
    }

    pub fn hud(&self) -> HudState {
        if self.destroyed {
            return HudState {
                title: "GARGANTUA COLLISION".to_string(),
                body: "The ship was torn apart by the event horizon.".to_string(),
                phase: GamePhase::Destroyed.label().to_string(),
                speed: 0.0,
                distance: distance(self.position, gargantua_position()),
                target: "GARGANTUA".to_string(),
                prompt: "PRESS R TO RESTART NEAR EARTH".to_string(),
                warning: "CRITICAL".to_string(),
                scan_progress: 0.0,
            };
        }

        if let Some(body_index) = self.landed_body {
            let body = &self.bodies[body_index];
            return HudState {
                title: format!("SURFACE: {}", body.label),
                body: landing_description(body.kind).to_string(),
                phase: GamePhase::Landed.label().to_string(),
                speed: 0.0,
                distance: 0.0,
                target: body.label.to_string(),
                prompt: "PRESS R TO TAKE OFF".to_string(),
                warning: String::new(),
                scan_progress: 0.0,
            };
        }

        let (target_name, target_distance, target_description) =
            if let Some((index, distance)) = self.nearest_visible_body() {
                let body = &self.bodies[index];
                (body.label.to_string(), distance, body.description.to_string())
            } else {
                ("DEEP SPACE".to_string(), 0.0, "No nearby navigation beacon.".to_string())
            };

        let wormhole_is_close = self.bodies.iter()
            .find(|body| body.kind == PlanetKind::Wormhole)
            .map(|body| (distance(body.position, self.position) - body.radius).max(0.0) < 380.0)
            .unwrap_or(false);
        let prompt = if self.phase == GamePhase::Transition {
            "WORMHOLE TRANSIT - HOLD STEADY".to_string()
        } else if !self.crossed_wormhole && wormhole_is_close {
            "PRESS SPACE TO CROSS THE WORMHOLE".to_string()
        } else if let Some((index, distance)) = self.nearest_landable() {
            if distance < 220.0 {
                format!("PRESS E TO LAND ON {}", self.bodies[index].label)
            } else {
                "FREE FLIGHT: STEER AND APPROACH A LANDABLE WORLD".to_string()
            }
        } else if target_name == "THE SUN" {
            "THE SUN IS A VISUAL LANDMARK, NOT A LANDING TARGET".to_string()
        } else {
            "FREE FLIGHT: W/S THRUST, A/D STEER, SHIFT BOOST".to_string()
        };

        let warning = if self.crossed_wormhole {
            let distance_to_black_hole = distance(self.position, gargantua_position());
            if distance_to_black_hole < GARGANTUA_RADIUS * 3.25 { "GRAVITY WARNING".to_string() } else { String::new() }
        } else {
            String::new()
        };

        HudState {
            title: target_name.clone(),
            body: target_description,
            phase: self.phase.label().to_string(),
            speed: self.speed,
            distance: target_distance,
            target: target_name,
            prompt,
            warning,
            scan_progress: 0.0,
        }
    }
}

fn landing_description(kind: PlanetKind) -> &'static str {
    match kind {
        PlanetKind::Earth => "Blue atmospheric horizon and distant cloud bands.",
        PlanetKind::Moon => "Airless grey regolith, sharp craters, and Earth on the horizon.",
        PlanetKind::Mars => "Amber sky, scattered basalt rocks, and a cold horizon.",
        PlanetKind::MillerWater => "A shallow reflective ocean with slow tidal walls.",
        PlanetKind::MannIce => "White-grey ice flats, frozen ridges, and drifting snow.",
        PlanetKind::EdmundHabitable => "Rock, moss-green vegetation, and warm light.",
        _ => "Surface contact established.",
    }
}

fn damp(current: f32, target: f32, lambda: f32, dt: f32) -> f32 {
    current + (target - current) * (1.0 - (-lambda * dt).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_starts_facing_negative_z() {
        let game = Game::new();
        assert!(game.forward().z < -0.99);
    }

    #[test]
    fn earth_is_visible_at_start() {
        let game = Game::new();
        let earth = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap();
        assert!(game.body_is_visible(earth));
    }

    #[test]
    fn restart_resets_wormhole_state() {
        let mut game = Game::new();
        game.crossed_wormhole = true;
        game.destroyed = true;
        game.restart();
        assert!(!game.crossed_wormhole);
        assert!(!game.destroyed);
        assert_eq!(game.phase, GamePhase::Launch);
    }
}
