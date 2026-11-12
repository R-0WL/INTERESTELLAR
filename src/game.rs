//! Journey data and flight simulation, separated from drawing code.

use crate::math::{clamp, cross, distance, dot, length, normalize, vec3, Vec3};

/// Current visual scale: each major celestial body uses 15 times its base
/// radius (rather than the previous 4x pass). The ship intentionally stays small.
pub const PLANET_SCALE: f32 = 15.0;
/// One astronomical unit is compressed to this many game-world units.
/// Increasing it alongside PLANET_SCALE preserves generous clearance between
/// the enlarged planets while keeping the real relative orbital distances.
pub const AU_SCALE: f32 = 30_000.0;
pub const BOOST_MAX_SPEED: f32 = 1_000.0;
pub const SUPERBOOST_MAX_SPEED: f32 = 2_500.0;

/// Stylized orbital radii around Gargantua. They leave a large empty central
/// volume for a dedicated black-hole OBJ and keep the planets outside its disk.
pub const GARGANTUA_MILLER_ORBIT: f32 = 85_000.0;
pub const GARGANTUA_MANN_ORBIT: f32 = 155_000.0;
pub const GARGANTUA_EDMUND_ORBIT: f32 = 235_000.0;
/// Radial clearance between the outer accretion-disk edge and Miller's orbit.
/// The simulation uses world units (not literal physical kilometres).
pub const GARGANTUA_DISK_ORBIT_GAP: f32 = 1_600.0;
/// The supplied black-hole OBJ has a normalized outer disk radius of 1.0 and
/// an event-horizon radius of approximately 0.425 of that disk radius.
pub const GARGANTUA_DISK_RADIUS: f32 = GARGANTUA_MILLER_ORBIT - GARGANTUA_DISK_ORBIT_GAP;
pub const GARGANTUA_CORE_TO_DISK_RATIO: f32 = 0.425;
pub const EARTH_ORBIT_PERIOD_SECONDS: f32 = 300.0;
pub const ASTEROID_BELT_INNER_AU: f32 = 2.1;
pub const ASTEROID_BELT_OUTER_AU: f32 = 3.3;
pub const GARGANTUA_RADIUS: f32 = GARGANTUA_DISK_RADIUS * GARGANTUA_CORE_TO_DISK_RATIO;
pub fn gargantua_position() -> Vec3 { vec3(0.0, -112.0, -1_125_000.0) }
/// Enter the second system outside Edmund's orbit, then fly inward toward the
/// central black hole and its widely spaced orbital worlds.
pub fn wormhole_exit() -> Vec3 { gargantua_position() + vec3(0.0, 12.0, 275_000.0) }

/// The wormhole stays beyond Saturn's enlarged rings and accompanies the planet.
fn wormhole_saturn_offset() -> Vec3 { vec3(18_000.0, 3_900.0, 0.0) }

fn solar_z(orbital_radius_au: f32) -> f32 {
    AU_SCALE - orbital_radius_au * AU_SCALE
}

fn kepler_period_seconds(orbital_radius_au: f32) -> f32 {
    EARTH_ORBIT_PERIOD_SECONDS * orbital_radius_au.powf(1.5)
}

fn angular_speed_for_period(period_seconds: f32) -> f32 {
    std::f32::consts::TAU / period_seconds
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetKind {
    Mercury,
    Venus,
    Earth,
    Sun,
    Moon,
    Mars,
    AsteroidBelt,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
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
    pub description: &'static str,
    pub reveal_distance: f32,
}

pub fn journey_bodies() -> Vec<SpaceBody> {
    // Planet positions are initialized on the same radial line to make the
    // whole route easy to read from the starting point. Their orbital radii
    // follow approximate real AU ratios, compressed by AU_SCALE.
    let belt_center_au = (ASTEROID_BELT_INNER_AU + ASTEROID_BELT_OUTER_AU) * 0.5;
    vec![
        SpaceBody {
            id: "earth", label: "EARTH", kind: PlanetKind::Earth,
            position: vec3(0.0, 0.0, solar_z(1.0)), radius: 180.0 * PLANET_SCALE,
            description: "Blue home world. The orbital reference completes one revolution every five minutes.",
            reveal_distance: 18_000.0,
        },
        SpaceBody {
            id: "moon", label: "MOON", kind: PlanetKind::Moon,
            position: vec3(6_500.0, 0.0, solar_z(1.0)), radius: 54.0 * PLANET_SCALE,
            description: "A small grey companion world, cratered and airless.",
            reveal_distance: 10_000.0,
        },
        SpaceBody {
            id: "mercury", label: "MERCURY", kind: PlanetKind::Mercury,
            position: vec3(0.0, 0.0, solar_z(0.387)), radius: 70.0 * PLANET_SCALE,
            description: "The innermost rocky planet, covered in ancient impact craters.",
            reveal_distance: 16_000.0,
        },
        SpaceBody {
            id: "venus", label: "VENUS", kind: PlanetKind::Venus,
            position: vec3(0.0, 0.0, solar_z(0.723)), radius: 171.0 * PLANET_SCALE,
            description: "A dense golden atmosphere hides the volcanic surface below.",
            reveal_distance: 18_000.0,
        },
        SpaceBody {
            id: "mars", label: "MARS", kind: PlanetKind::Mars,
            position: vec3(0.0, 0.0, solar_z(1.524)), radius: 105.0 * PLANET_SCALE,
            description: "Dusty red terrain, a thin atmosphere, and polar ice haze.",
            reveal_distance: 50_000.0,
        },
        SpaceBody {
            id: "asteroids", label: "ASTEROID BELT", kind: PlanetKind::AsteroidBelt,
            position: vec3(0.0, 0.0, solar_z(belt_center_au)), radius: 190.0 * PLANET_SCALE,
            description: "A complete annular field of rocky fragments between Mars and Jupiter.",
            reveal_distance: 55_000.0,
        },
        SpaceBody {
            id: "jupiter", label: "JUPITER", kind: PlanetKind::Jupiter,
            position: vec3(0.0, 0.0, solar_z(5.203)), radius: 350.0 * PLANET_SCALE,
            description: "An immense banded gas giant with a red storm system.",
            reveal_distance: 50_000.0,
        },
        SpaceBody {
            id: "sun", label: "THE SUN", kind: PlanetKind::Sun,
            position: vec3(0.0, 0.0, AU_SCALE), radius: 300.0 * PLANET_SCALE,
            description: "The star at the center of the Solar System.",
            reveal_distance: f32::INFINITY,
        },
        SpaceBody {
            id: "saturn", label: "SATURN", kind: PlanetKind::Saturn,
            position: vec3(0.0, 0.0, solar_z(9.537)), radius: 300.0 * PLANET_SCALE,
            description: "Pale gold atmosphere and a dense field of icy rings.",
            reveal_distance: 60_000.0,
        },
        SpaceBody {
            id: "uranus", label: "URANUS", kind: PlanetKind::Uranus,
            position: vec3(0.0, 0.0, solar_z(19.191)), radius: 263.0 * PLANET_SCALE,
            description: "A distant ice giant with a blue-green atmosphere and a tilted axis.",
            reveal_distance: 85_000.0,
        },
        SpaceBody {
            id: "neptune", label: "NEPTUNE", kind: PlanetKind::Neptune,
            position: vec3(0.0, 0.0, solar_z(30.069)), radius: 255.0 * PLANET_SCALE,
            description: "A deep blue ice giant at the edge of the planetary region.",
            reveal_distance: 110_000.0,
        },
        SpaceBody {
            id: "wormhole", label: "SOLAR SYSTEM WORMHOLE", kind: PlanetKind::Wormhole,
            position: vec3(18_000.0, 3_900.0, solar_z(9.537)), radius: 38.0 * PLANET_SCALE,
            description: "A stable spacetime anomaly travelling beside Saturn.",
            reveal_distance: 75_000.0,
        },
        SpaceBody {
            id: "docking", label: "ENDURANCE DOCKING", kind: PlanetKind::Docking,
            position: gargantua_position() + vec3(1_500.0, 12.0, 265_000.0), radius: 20.0 * PLANET_SCALE,
            description: "Docking rotation matched. A new galaxy lies ahead.",
            reveal_distance: 45_000.0,
        },
        SpaceBody {
            id: "gargantua", label: "GARGANTUA", kind: PlanetKind::Gargantua,
            position: gargantua_position(), radius: GARGANTUA_RADIUS,
            description: "Event horizon, photon ring, and a violent accretion disk.",
            reveal_distance: 320_000.0,
        },
        SpaceBody {
            id: "miller", label: "MILLER'S PLANET", kind: PlanetKind::MillerWater,
            position: gargantua_position() + vec3(0.0, -58.0, GARGANTUA_MILLER_ORBIT), radius: 135.0 * PLANET_SCALE,
            description: "A water world close enough to feel Gargantua's pull.",
            reveal_distance: 120_000.0,
        },
        SpaceBody {
            id: "mann", label: "MANN'S PLANET", kind: PlanetKind::MannIce,
            position: gargantua_position() + vec3(0.0, -132.0, GARGANTUA_MANN_ORBIT), radius: 115.0 * PLANET_SCALE,
            description: "Frozen atmosphere, fractured ice crust, and a weak beacon.",
            reveal_distance: 160_000.0,
        },
        SpaceBody {
            id: "edmund", label: "EDMUND'S PLANET", kind: PlanetKind::EdmundHabitable,
            position: gargantua_position() + vec3(0.0, 120.0, GARGANTUA_EDMUND_ORBIT), radius: 125.0 * PLANET_SCALE,
            description: "Habitable signatures detected. Journey endpoint acquired.",
            reveal_distance: 70_000.0,
        },
    ]
}

/// One body moving on a circular orbit in the XZ plane.
#[derive(Clone, Debug)]
struct OrbitalMotion {
    body_kind: PlanetKind,
    center_kind: PlanetKind,
    radius: f32,
    y_offset: f32,
    phase: f32,
    initial_phase: f32,
    angular_speed: f32,
}

fn build_orbits(bodies: &[SpaceBody]) -> Vec<OrbitalMotion> {
    let definitions = [
        // Kepler's third law: P^2 is proportional to a^3. With Earth set to
        // exactly 300 seconds, every other solar planet keeps that period ratio.
        (PlanetKind::Mercury, PlanetKind::Sun, angular_speed_for_period(kepler_period_seconds(0.387))),
        (PlanetKind::Venus, PlanetKind::Sun, angular_speed_for_period(kepler_period_seconds(0.723))),
        (PlanetKind::Earth, PlanetKind::Sun, angular_speed_for_period(EARTH_ORBIT_PERIOD_SECONDS)),
        (PlanetKind::Mars, PlanetKind::Sun, angular_speed_for_period(kepler_period_seconds(1.524))),
        (PlanetKind::Jupiter, PlanetKind::Sun, angular_speed_for_period(kepler_period_seconds(5.203))),
        (PlanetKind::Saturn, PlanetKind::Sun, angular_speed_for_period(kepler_period_seconds(9.537))),
        (PlanetKind::Uranus, PlanetKind::Sun, angular_speed_for_period(kepler_period_seconds(19.191))),
        (PlanetKind::Neptune, PlanetKind::Sun, angular_speed_for_period(kepler_period_seconds(30.069))),
        // The Moon's sidereal period is scaled by its real ratio to Earth's year.
        (PlanetKind::Moon, PlanetKind::Earth, angular_speed_for_period(EARTH_ORBIT_PERIOD_SECONDS * (27.321_661_f32 / 365.256_363_f32))),
        // The second-system planets are deliberately far from Gargantua so a
        // custom OBJ black-hole model and its compact accretion rings have room.
        // Their periods follow Kepler-like ratios from the innermost orbit.
        (PlanetKind::MillerWater, PlanetKind::Gargantua, angular_speed_for_period(2_100.0)),
        (PlanetKind::MannIce, PlanetKind::Gargantua, angular_speed_for_period(2_100.0 * (GARGANTUA_MANN_ORBIT / GARGANTUA_MILLER_ORBIT).powf(1.5))),
        (PlanetKind::EdmundHabitable, PlanetKind::Gargantua, angular_speed_for_period(2_100.0 * (GARGANTUA_EDMUND_ORBIT / GARGANTUA_MILLER_ORBIT).powf(1.5))),
    ];

    definitions
        .iter()
        .filter_map(|(body_kind, center_kind, angular_speed)| {
            let body = bodies.iter().find(|body| body.kind == *body_kind)?;
            let center = bodies.iter().find(|body| body.kind == *center_kind)?;
            let offset = body.position - center.position;
            let radius = (offset.x * offset.x + offset.z * offset.z).sqrt();
            let phase = offset.z.atan2(offset.x);
            Some(OrbitalMotion {
                body_kind: *body_kind,
                center_kind: *center_kind,
                radius,
                y_offset: offset.y,
                phase,
                initial_phase: phase,
                angular_speed: *angular_speed,
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InputState {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub pitch_up: bool,
    pub pitch_down: bool,
    /// Hold Shift while thrusting to enable the regular boost.
    pub boost: bool,
    /// Hold B together with Shift and W to engage the 2500-speed superboost.
    pub superboost: bool,
    /// Toggle the near-body assisted orbit when a body is in range.
    pub orbit_pressed: bool,
    pub wormhole_pressed: bool,
    pub restart_pressed: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverviewOrientation {
    Top,
    Side,
}

#[derive(Clone, Copy, Debug)]
pub struct SystemOverview {
    pub center: PlanetKind,
    pub orientation: OverviewOrientation,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub bodies: Vec<SpaceBody>,
    orbits: Vec<OrbitalMotion>,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    /// Exposed to the renderer so boost visuals remain separate from simulation.
    pub boost_active: bool,
    pub superboost_active: bool,
    pub phase: GamePhase,
    pub crossed_wormhole: bool,
    pub orbit_target: Option<usize>,
    pub yaw_rate: f32,
    pub pitch_rate: f32,
    pub overview: Option<SystemOverview>,
    pub destroyed: bool,
    pub transition_timer: f32,
    pub clock: f32,
    pub capture_notice: f32,
}

impl Game {
    pub fn new() -> Self {
        let bodies = journey_bodies();
        let orbits = build_orbits(&bodies);
        Self {
            bodies,
            orbits,
            // Start in the clear corridor between Earth and Venus, facing Earth.
            position: vec3(0.0, 80.0, 4_000.0),
            velocity: vec3(0.0, 0.0, -46.0),
            yaw: 0.0,
            pitch: -0.03,
            speed: 58.0,
            boost_active: false,
            superboost_active: false,
            phase: GamePhase::Launch,
            crossed_wormhole: false,
            orbit_target: None,
            yaw_rate: 0.0,
            pitch_rate: 0.0,
            overview: None,
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
        // Split slower frames into fixed-size simulation steps. This keeps
        // steering, braking, gravity and orbit corrections consistent on
        // machines that deliver frames at different rates.
        let frame_dt = dt.clamp(0.0, 0.25);
        let steps = ((frame_dt * 60.0 - 0.0001).ceil() as usize).max(1);
        let step_dt = frame_dt / steps as f32;
        for index in 0..steps {
            let mut step_input = input;
            if index > 0 {
                step_input.orbit_pressed = false;
                step_input.wormhole_pressed = false;
                step_input.restart_pressed = false;
                step_input.screenshot_pressed = false;
            }
            self.update_step(step_dt, step_input);
        }
    }

    fn update_step(&mut self, dt: f32, input: InputState) {
        let dt = dt.clamp(0.0, 0.05);
        self.clock += dt;
        self.update_orbits(dt);
        self.capture_notice = (self.capture_notice - dt).max(0.0);
        if input.screenshot_pressed {
            self.capture_notice = 2.5;
        }

        if input.restart_pressed && !self.destroyed {
            self.restart();
            return;
        }

        if self.destroyed {
            self.boost_active = false;
            self.superboost_active = false;
            self.phase = GamePhase::Destroyed;
            self.speed = 0.0;
            self.velocity = vec3(0.0, 0.0, 0.0);
            if input.restart_pressed {
                self.restart();
            }
            return;
        }

        if self.phase == GamePhase::Transition {
            self.boost_active = false;
            self.superboost_active = false;
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

        // Progressive, time-based controls. All rates are expressed per second,
        // so the response is stable at 30, 60 or 144 FPS.
        let yaw_input = (input.left as i32 - input.right as i32) as f32;
        let pitch_input = (input.pitch_up as i32 - input.pitch_down as i32) as f32;
        self.yaw_rate = damp(self.yaw_rate, yaw_input * 1.10, 5.5, dt);
        self.pitch_rate = damp(self.pitch_rate, pitch_input * 0.82, 5.5, dt);
        self.yaw += self.yaw_rate * dt;
        self.pitch = clamp(self.pitch + self.pitch_rate * dt, -1.25, 1.25);

        let superboost_requested = input.boost && input.superboost && input.forward;
        let speed_limit = if superboost_requested {
            SUPERBOOST_MAX_SPEED
        } else if input.boost && input.forward {
            BOOST_MAX_SPEED
        } else {
            215.0
        };
        if input.forward {
            if self.speed > speed_limit {
                // Moving from superboost to a lower thrust mode eases down
                // instead of snapping instantly to the new mode's speed cap.
                self.speed = (self.speed - 190.0 * dt).max(speed_limit);
            } else {
                let acceleration = if superboost_requested { 820.0 } else if input.boost { 320.0 } else { 78.0 };
                self.speed = (self.speed + acceleration * dt).min(speed_limit);
            }
        } else if input.back {
            self.speed = (self.speed - 190.0 * dt).max(0.0);
        } else {
            // Mild coasting drag is exponential in elapsed time, not frame count.
            self.speed *= (-0.12 * dt).exp();
        }
        self.speed = self.speed.max(0.0);
        self.boost_active = input.boost && input.forward;
        self.superboost_active = superboost_requested;

        let forward = self.forward();
        let alignment = 1.0 - (-2.15 * dt).exp();
        let current_direction = if length(self.velocity) > 0.1 {
            normalize(self.velocity)
        } else {
            forward
        };
        // Smooth the course direction independently from the scalar speed.
        // This keeps acceleration at its configured units/second² instead of
        // accidentally damping it a second time through velocity alignment.
        let flight_direction = normalize(current_direction + (forward - current_direction) * alignment);
        self.velocity = flight_direction * self.speed;

        // Celestial gravity bends the trajectory continuously. The force is
        // intentionally stylized for the compressed world scale of the game.
        let gravity = self.gravity_acceleration();
        self.velocity += gravity * dt;

        if input.orbit_pressed {
            self.toggle_orbit_assist();
        }
        self.apply_orbit_assist(dt);
        if self.apply_body_avoidance(dt) {
            self.align_heading_to_velocity(dt);
        }

        // A requested thrust mode has a strict top speed. Gravity and the
        // navigation assists may bend the velocity vector, but cannot push a
        // boost mode beyond its advertised cap. With no thrust, gravity may
        // still accelerate the ship naturally.
        if input.forward && self.speed <= speed_limit {
            let current_speed = length(self.velocity);
            if current_speed > speed_limit {
                self.velocity = normalize(self.velocity) * speed_limit;
            }
        }
        self.speed = length(self.velocity).min(SUPERBOOST_MAX_SPEED);
        self.position += self.velocity * dt;

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

        self.phase = if self.nearest_visible_body().map(|(_, d)| d < 420.0).unwrap_or(false) {
            GamePhase::Flyby
        } else if self.crossed_wormhole && self.position.z < gargantua_position().z - 25_000.0 {
            GamePhase::Completed
        } else if self.crossed_wormhole && distance(self.position, gargantua_position()) > 18_000.0 {
            GamePhase::Arrival
        } else if self.position.z > -120.0 && !self.crossed_wormhole {
            GamePhase::Launch
        } else {
            GamePhase::Cruise
        };
    }

    fn body_belongs_to_active_system(&self, kind: PlanetKind) -> bool {
        let second_system = matches!(
            kind,
            PlanetKind::Docking | PlanetKind::Gargantua | PlanetKind::MillerWater |
                PlanetKind::MannIce | PlanetKind::EdmundHabitable
        );
        self.crossed_wormhole == second_system
    }

    fn is_solid_navigation_body(kind: PlanetKind) -> bool {
        !matches!(kind, PlanetKind::AsteroidBelt | PlanetKind::Wormhole | PlanetKind::Docking)
    }

    fn gravity_surface_strength(kind: PlanetKind) -> f32 {
        match kind {
            PlanetKind::Sun => 22.0,
            PlanetKind::Gargantua => 150.0,
            PlanetKind::Jupiter => 25.0,
            PlanetKind::Saturn => 18.0,
            PlanetKind::Earth | PlanetKind::Venus | PlanetKind::MillerWater => 15.0,
            PlanetKind::Uranus | PlanetKind::Neptune | PlanetKind::EdmundHabitable => 12.0,
            PlanetKind::Mercury | PlanetKind::Mars | PlanetKind::MannIce => 8.0,
            PlanetKind::Moon => 4.0,
            _ => 0.0,
        }
    }

    fn effective_gravity_radius(body: &SpaceBody) -> f32 {
        if body.kind == PlanetKind::Gargantua {
            GARGANTUA_RADIUS
        } else {
            body.radius
        }
    }

    fn gravity_acceleration(&self) -> Vec3 {
        let mut total = vec3(0.0, 0.0, 0.0);
        for body in &self.bodies {
            if !self.body_belongs_to_active_system(body.kind) || !Self::is_solid_navigation_body(body.kind) {
                continue;
            }
            let radius = Self::effective_gravity_radius(body).max(1.0);
            let offset = body.position - self.position;
            let center_distance = length(offset).max(radius * 0.75);
            let influence_radius = if body.kind == PlanetKind::Gargantua { radius * 9.0 } else { radius * 10.0 };
            if center_distance > influence_radius {
                continue;
            }
            let surface_g = Self::gravity_surface_strength(body.kind);
            let mu = surface_g * radius * radius;
            let fade = 1.0 - (center_distance / influence_radius).powi(2);
            let magnitude = (mu / (center_distance * center_distance)) * fade.max(0.0);
            total += normalize(offset) * magnitude;
        }
        let magnitude = length(total);
        if magnitude > 260.0 { normalize(total) * 260.0 } else { total }
    }

    fn collision_radius(body: &SpaceBody) -> f32 {
        match body.kind {
            // Gargantua's visible accretion disc is also treated as a no-fly
            // volume, rather than allowing the ship to pass through its mesh.
            PlanetKind::Gargantua => GARGANTUA_DISK_RADIUS * 0.96,
            PlanetKind::Sun => body.radius + 1_200.0,
            _ => body.radius + body.radius.max(350.0) * 0.10 + 280.0,
        }
    }

    fn toggle_orbit_assist(&mut self) {
        if self.orbit_target.is_some() {
            self.orbit_target = None;
            return;
        }
        let candidate = self.bodies.iter().enumerate()
            .filter(|(_, body)| self.body_belongs_to_active_system(body.kind) && Self::is_solid_navigation_body(body.kind))
            .map(|(index, body)| (index, distance(body.position, self.position) - Self::effective_gravity_radius(body)))
            .filter(|(index, surface_distance)| {
                let radius = Self::effective_gravity_radius(&self.bodies[*index]);
                *surface_distance >= 0.0 && *surface_distance <= (radius * 9.0).max(4_500.0)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        self.orbit_target = candidate.map(|(index, _)| index);
    }

    fn apply_orbit_assist(&mut self, dt: f32) {
        let Some(index) = self.orbit_target else { return; };
        let Some(body) = self.bodies.get(index) else {
            self.orbit_target = None;
            return;
        };
        let offset = self.position - body.position;
        let radial_distance = length(offset);
        let effective_radius = Self::effective_gravity_radius(body);
        if radial_distance > (effective_radius * 11.0).max(6_000.0) || radial_distance < effective_radius * 1.02 {
            self.orbit_target = None;
            return;
        }

        let radial = normalize(offset);
        let mut tangent = normalize(cross(vec3(0.0, 1.0, 0.0), radial));
        if length(tangent) < 0.001 {
            tangent = normalize(cross(vec3(1.0, 0.0, 0.0), radial));
        }
        if dot(tangent, self.velocity) < 0.0 {
            tangent = -tangent;
        }
        let radial_velocity = dot(self.velocity, radial);
        let target_velocity = tangent * self.speed.max(55.0) + radial * (radial_velocity * 0.06);
        let blend = 1.0 - (-0.62 * dt).exp();
        self.velocity += (target_velocity - self.velocity) * blend;

        // Add the centripetal component needed to keep the assisted path curved.
        let tangential_speed = length(self.velocity - radial * dot(self.velocity, radial));
        self.velocity -= radial * (tangential_speed * tangential_speed / radial_distance.max(1.0)) * dt;
    }

    fn apply_body_avoidance(&mut self, dt: f32) -> bool {
        let velocity_direction = normalize(self.velocity);
        let velocity_magnitude = length(self.velocity);
        if velocity_magnitude < 0.1 {
            return false;
        }
        let mut avoidance_engaged = false;
        let lookahead_distance = (velocity_magnitude * 2.4).max(4_000.0).min(35_000.0);
        let segment = velocity_direction * lookahead_distance;
        let segment_length_squared = dot(segment, segment).max(1.0);

        for body in &self.bodies {
            if !self.body_belongs_to_active_system(body.kind) || !Self::is_solid_navigation_body(body.kind) {
                continue;
            }
            let safe_radius = Self::collision_radius(body);
            let to_center = body.position - self.position;
            let along = dot(to_center, velocity_direction);
            if along < -safe_radius || along > lookahead_distance + safe_radius {
                continue;
            }
            let t = clamp(dot(to_center, segment) / segment_length_squared, 0.0, 1.0);
            let closest = self.position + segment * t;
            let center_offset = closest - body.position;
            let clearance = length(center_offset);
            let response_distance = safe_radius + (velocity_magnitude * 0.75).max(1_000.0);
            if clearance >= response_distance {
                continue;
            }
            avoidance_engaged = true;

            let radial = if clearance > 0.001 {
                center_offset / clearance
            } else {
                normalize(self.position - body.position)
            };
            let mut tangent = normalize(cross(velocity_direction, radial));
            if length(tangent) < 0.001 {
                tangent = normalize(cross(vec3(0.0, 1.0, 0.0), radial));
            }
            if length(tangent) < 0.001 {
                tangent = vec3(1.0, 0.0, 0.0);
            }
            if dot(tangent, self.right()) < 0.0 {
                tangent = -tangent;
            }
            let urgency = clamp((response_distance - clearance) / response_distance.max(1.0), 0.0, 1.0);
            self.velocity += (radial * 145.0 + tangent * 110.0) * (urgency * dt);

            // Last-resort geometric guard for unusually large frame steps or
            // superboost: the ship is repositioned on the safe shell, not bounced.
            let predicted_position = self.position + self.velocity * dt;
            let future_offset = predicted_position - body.position;
            let future_distance = length(future_offset);
            if future_distance < safe_radius {
                let outward = if future_distance > 0.001 { future_offset / future_distance } else { radial };
                self.position = body.position + outward * (safe_radius + velocity_magnitude * dt * 1.5);
                let inward_speed = dot(self.velocity, outward);
                if inward_speed < 0.0 {
                    self.velocity -= outward * inward_speed;
                }
                self.velocity += tangent * (velocity_magnitude * 0.18);
            }

            // Also recover if a camera placement begins slightly inside a body.
            let current_offset = self.position - body.position;
            let current_distance = length(current_offset);
            if current_distance < safe_radius {
                let outward = if current_distance > 0.001 { current_offset / current_distance } else { radial };
                self.position = body.position + outward * safe_radius;
                let inward_speed = dot(self.velocity, outward);
                if inward_speed < 0.0 {
                    self.velocity -= outward * inward_speed;
                }
            }
        }
        avoidance_engaged
    }

    fn align_heading_to_velocity(&mut self, dt: f32) {
        let course = normalize(self.velocity);
        if length(course) < 0.001 {
            return;
        }
        let desired_yaw = (-course.x).atan2(-course.z);
        let yaw_delta = (desired_yaw - self.yaw + std::f32::consts::PI)
            .rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        self.yaw += yaw_delta.clamp(-1.25 * dt, 1.25 * dt);
        let desired_pitch = course.y.clamp(-1.0, 1.0).asin();
        self.pitch = damp(self.pitch, desired_pitch, 2.8, dt).clamp(-1.25, 1.25);
    }

    pub fn place_named(&mut self, target: &str) -> Result<(), String> {
        let key = normalize_place_name(target);
        let overview = match key.as_str() {
            "galaxy1-top" | "galaxy-1-top" | "galaxia1-superior" | "vista-superior-galaxia-1" => Some(SystemOverview {
                center: PlanetKind::Sun,
                orientation: OverviewOrientation::Top,
            }),
            "galaxy1-side" | "galaxy-1-side" | "galaxia1-lateral" | "vista-lateral-galaxia-1" => Some(SystemOverview {
                center: PlanetKind::Sun,
                orientation: OverviewOrientation::Side,
            }),
            "galaxy2-top" | "galaxy-2-top" | "galaxia2-superior" | "vista-superior-galaxia-2" => Some(SystemOverview {
                center: PlanetKind::Gargantua,
                orientation: OverviewOrientation::Top,
            }),
            "galaxy2-side" | "galaxy-2-side" | "galaxia2-lateral" | "vista-lateral-galaxia-2" => Some(SystemOverview {
                center: PlanetKind::Gargantua,
                orientation: OverviewOrientation::Side,
            }),
            _ => None,
        };
        if let Some(overview) = overview {
            self.crossed_wormhole = overview.center == PlanetKind::Gargantua;
            self.overview = Some(overview);
            self.orbit_target = None;
            self.destroyed = false;
            self.phase = GamePhase::Cruise;
            return Ok(());
        }

        let kind = match key.as_str() {
            "mercury" | "mercurio" => PlanetKind::Mercury,
            "venus" => PlanetKind::Venus,
            "earth" | "tierra" => PlanetKind::Earth,
            "moon" | "luna" => PlanetKind::Moon,
            "mars" | "marte" => PlanetKind::Mars,
            "jupiter" | "jupiter-gas" => PlanetKind::Jupiter,
            "saturn" | "saturno" => PlanetKind::Saturn,
            "uranus" | "urano" => PlanetKind::Uranus,
            "neptune" | "neptuno" => PlanetKind::Neptune,
            "sun" | "sol" => PlanetKind::Sun,
            "wormhole" | "agujero-de-gusano" | "agujero-gusano" => PlanetKind::Wormhole,
            "gargantua" | "black-hole" | "agujero-negro" => PlanetKind::Gargantua,
            "endurance" | "endurance-docking" => PlanetKind::Docking,
            "miller" | "millers-planet" | "planeta-de-miller" => PlanetKind::MillerWater,
            "mann" | "manns-planet" | "planeta-de-mann" => PlanetKind::MannIce,
            "edmund" | "edmunds-planet" | "planeta-de-edmund" => PlanetKind::EdmundHabitable,
            _ => return Err(format!("Lugar desconocido: '{target}'. Usa --help para ver la lista.")),
        };

        let body = self.bodies.iter().find(|body| body.kind == kind)
            .ok_or_else(|| format!("No se encontró el cuerpo solicitado: {target}"))?;
        let center = body.position;
        let radius = body.radius;
        let second_system = matches!(kind, PlanetKind::Docking | PlanetKind::Gargantua | PlanetKind::MillerWater | PlanetKind::MannIce | PlanetKind::EdmundHabitable);
        self.crossed_wormhole = second_system;
        self.overview = None;
        self.orbit_target = None;
        self.destroyed = false;
        self.phase = GamePhase::Launch;

        let central_light = if second_system {
            gargantua_position()
        } else {
            self.bodies.iter().find(|item| item.kind == PlanetKind::Sun).map(|item| item.position).unwrap_or(vec3(0.0, 0.0, 0.0))
        };
        let mut sun_facing = if kind == PlanetKind::Sun {
            vec3(0.0, 0.0, -1.0)
        } else if kind == PlanetKind::Gargantua {
            vec3(0.0, 0.0, 1.0)
        } else {
            normalize(central_light - center)
        };
        if length(sun_facing) < 0.001 {
            sun_facing = vec3(0.0, 0.0, 1.0);
        }
        let stand_off = if kind == PlanetKind::Gargantua {
            GARGANTUA_DISK_RADIUS * 1.55
        } else {
            let preferred = radius * 2.5 + 1_400.0;
            if second_system {
                // Miller's orbit sits only 1,600 units beyond the visible
                // disk edge. Keep the Sun-facing spawn between the planet's
                // collision shell and Gargantua's no-fly shell instead of
                // placing the ship inside the black-hole safety volume.
                let core_distance = distance(center, gargantua_position());
                let outer_clearance = GARGANTUA_DISK_RADIUS * 0.96 + 1_200.0;
                let max_toward_core = (core_distance - outer_clearance).max(radius + 1_000.0);
                preferred.min(max_toward_core)
            } else {
                preferred
            }
        };
        self.position = center + sun_facing * stand_off;
        let direction = normalize(center - self.position);
        self.yaw = (-direction.x).atan2(-direction.z);
        self.pitch = direction.y.clamp(-1.0, 1.0).asin();
        self.yaw_rate = 0.0;
        self.pitch_rate = 0.0;
        self.speed = 58.0;
        self.velocity = direction * self.speed;
        self.boost_active = false;
        self.superboost_active = false;
        Ok(())
    }

    pub fn is_overview(&self) -> bool {
        self.overview.is_some()
    }

    pub fn camera_up(&self) -> Vec3 {
        match self.overview.map(|view| view.orientation) {
            Some(OverviewOrientation::Top) => vec3(0.0, 0.0, -1.0),
            _ => vec3(0.0, 1.0, 0.0),
        }
    }

    /// Advance all orbital bodies before flight, gravity and rendering.
    fn update_orbits(&mut self, dt: f32) {

        for orbit in &mut self.orbits {
            orbit.phase = (orbit.phase + orbit.angular_speed * dt).rem_euclid(std::f32::consts::TAU);
            let Some(center_position) = self.bodies.iter()
                .find(|body| body.kind == orbit.center_kind)
                .map(|body| body.position) else {
                    continue;
                };
            let (sin_phase, cos_phase) = orbit.phase.sin_cos();
            let position = center_position + vec3(
                orbit.radius * cos_phase,
                orbit.y_offset,
                orbit.radius * sin_phase,
            );
            if let Some(body) = self.bodies.iter_mut().find(|body| body.kind == orbit.body_kind) {
                body.position = position;
            }
        }

        // The wormhole is a companion landmark, not a ninth planet: keep its
        // local offset fixed so it remains visibly beside Saturn throughout the orbit.
        if let Some(saturn_position) = self.bodies.iter()
            .find(|body| body.kind == PlanetKind::Saturn)
            .map(|body| body.position)
        {
            if let Some(wormhole) = self.bodies.iter_mut()
                .find(|body| body.kind == PlanetKind::Wormhole)
            {
                wormhole.position = saturn_position + wormhole_saturn_offset();
            }
        }

    }

    pub fn restart(&mut self) {
        // Restart the orbital clock as well as the ship, returning the planets
        // to their authored starting positions and phases.
        for orbit in &mut self.orbits {
            orbit.phase = orbit.initial_phase;
        }
        self.update_orbits(0.0);
        self.position = vec3(0.0, 80.0, 4_000.0);
        self.velocity = vec3(0.0, 0.0, -46.0);
        self.yaw = 0.0;
        self.pitch = -0.03;
        self.speed = 58.0;
        self.boost_active = false;
        self.superboost_active = false;
        self.phase = GamePhase::Launch;
        self.crossed_wormhole = false;
        self.orbit_target = None;
        self.yaw_rate = 0.0;
        self.pitch_rate = 0.0;
        self.overview = None;
        self.destroyed = false;
        self.transition_timer = 0.0;
    }

    pub fn body_is_visible(&self, body: &SpaceBody) -> bool {
        let in_solar_system = matches!(
            body.kind,
            PlanetKind::Mercury | PlanetKind::Venus | PlanetKind::Earth | PlanetKind::Sun |
            PlanetKind::Moon | PlanetKind::Mars | PlanetKind::AsteroidBelt | PlanetKind::Jupiter |
            PlanetKind::Saturn | PlanetKind::Uranus | PlanetKind::Neptune | PlanetKind::Wormhole
        );
        let in_gargantua_system = matches!(
            body.kind,
            PlanetKind::Docking | PlanetKind::Gargantua | PlanetKind::MillerWater |
            PlanetKind::MannIce | PlanetKind::EdmundHabitable
        );
        if let Some(view) = self.overview {
            return match view.center {
                PlanetKind::Sun => in_solar_system,
                PlanetKind::Gargantua => in_gargantua_system,
                _ => false,
            };
        }
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


    pub fn camera(&self) -> (Vec3, Vec3) {
        if let Some(view) = self.overview {
            let center = self.bodies.iter().find(|body| body.kind == view.center)
                .map(|body| body.position).unwrap_or(vec3(0.0, 0.0, 0.0));
            let distance = if view.center == PlanetKind::Sun { 1_950_000.0 } else { 520_000.0 };
            let offset = match view.orientation {
                OverviewOrientation::Top => vec3(0.0, distance, 0.0),
                OverviewOrientation::Side => vec3(distance, distance * 0.025, 0.0),
            };
            return (center + offset, center);
        }
        if self.destroyed {
            let eye = self.position + vec3(18.0, 12.0, 26.0);
            return (eye, gargantua_position());
        }
        // Keep the chase camera aligned with the ship's nose instead of its
        // drifting velocity vector. This preserves the aft view while steering.
        let direction = self.forward();
        let camera_right = normalize(cross(direction, vec3(0.0, 1.0, 0.0)));
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
        } else if let Some(index) = self.orbit_target {
            format!("ORBIT ASSIST: {}  |  PRESS E TO EXIT", self.bodies[index].label)
        } else if target_name == "THE SUN" {
            "SUN GRAVITY ACTIVE: HOLD COURSE AND ORBIT SAFELY".to_string()
        } else if self.superboost_active {
            "SUPERBOOST ONLINE: 2500 MAX SPEED".to_string()
        } else if self.boost_active {
            "BOOST ACTIVE: HOLD W + SHIFT".to_string()
        } else {
            "FREE FLIGHT: W/S THRUST, SHIFT BOOST, SHIFT+B SUPERBOOST".to_string()
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


fn normalize_place_name(value: &str) -> String {
    value.trim().trim_start_matches('[').trim_end_matches(']').to_ascii_lowercase()
        .replace('á', "a").replace('é', "e").replace('í', "i")
        .replace('ó', "o").replace('ú', "u").replace('ñ', "n")
        .replace('_', "-").replace(' ', "-")
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
    fn giant_earth_and_moon_have_clearance_between_surfaces() {
        let game = Game::new();
        let earth = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap();
        let moon = game.bodies.iter().find(|body| body.kind == PlanetKind::Moon).unwrap();
        assert!(earth.radius >= 2_600.0);
        assert!(moon.radius >= 800.0);
        assert!(distance(earth.position, moon.position) > earth.radius + moon.radius);
    }

    #[test]
    fn ship_starts_outside_the_enlarged_earth() {
        let game = Game::new();
        let earth = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap();
        assert!(distance(game.position, earth.position) > earth.radius);
    }

    #[test]
    fn orbital_bodies_move_slowly_but_visibly() {
        let mut game = Game::new();
        let before = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap().position;
        for _ in 0..120 {
            game.update(0.05, InputState::default());
        }
        let after = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap().position;
        // At 1 AU and a five-minute period, six simulated seconds move Earth
        // about 7.2 degrees along its 30,000-unit orbit.
        assert!(distance(before, after) > 3_700.0);
        assert!(distance(before, after) < 3_800.0);
    }

    #[test]
    fn orbital_radii_stay_constant_while_planets_move() {
        let mut game = Game::new();
        let sun = game.bodies.iter().find(|body| body.kind == PlanetKind::Sun).unwrap().position;
        let initial = game.bodies.iter().find(|body| body.kind == PlanetKind::Jupiter).unwrap().position;
        let initial_radius = distance(sun, initial);
        for _ in 0..240 {
            game.update(0.05, InputState::default());
        }
        let current = game.bodies.iter().find(|body| body.kind == PlanetKind::Jupiter).unwrap().position;
        let current_radius = distance(sun, current);
        assert!((initial_radius - current_radius).abs() < 1.0);
    }

    #[test]
    fn expanded_planets_have_clear_spacing() {
        let game = Game::new();
        let adjacent = [
            PlanetKind::Mercury, PlanetKind::Venus, PlanetKind::Earth, PlanetKind::Mars,
            PlanetKind::AsteroidBelt, PlanetKind::Jupiter, PlanetKind::Saturn,
            PlanetKind::Uranus, PlanetKind::Neptune,
        ];
        for pair in adjacent.windows(2) {
            let first = game.bodies.iter().find(|body| body.kind == pair[0]).unwrap();
            let second = game.bodies.iter().find(|body| body.kind == pair[1]).unwrap();
            assert!(distance(first.position, second.position) > first.radius + second.radius,
                "{} and {} overlap", first.label, second.label);
        }
    }

    #[test]
    fn starting_ship_is_clear_of_inner_planets() {
        let game = Game::new();
        for kind in [PlanetKind::Mercury, PlanetKind::Venus, PlanetKind::Earth, PlanetKind::Moon, PlanetKind::Sun] {
            let body = game.bodies.iter().find(|body| body.kind == kind).unwrap();
            assert!(distance(game.position, body.position) > body.radius, "ship starts inside {}", body.label);
        }
    }

    #[test]
    fn all_eight_solar_planets_are_present() {
        let game = Game::new();
        for kind in [
            PlanetKind::Mercury, PlanetKind::Venus, PlanetKind::Earth, PlanetKind::Mars,
            PlanetKind::Jupiter, PlanetKind::Saturn, PlanetKind::Uranus, PlanetKind::Neptune,
        ] {
            assert!(game.bodies.iter().any(|body| body.kind == kind), "missing {kind:?}");
        }
    }

    #[test]
    fn every_solar_planet_has_a_sun_orbit() {
        let game = Game::new();
        for kind in [
            PlanetKind::Mercury, PlanetKind::Venus, PlanetKind::Earth, PlanetKind::Mars,
            PlanetKind::Jupiter, PlanetKind::Saturn, PlanetKind::Uranus, PlanetKind::Neptune,
        ] {
            let orbit = game.orbits.iter().find(|orbit| orbit.body_kind == kind)
                .expect("every Solar System planet must move");
            assert_eq!(orbit.center_kind, PlanetKind::Sun);
            assert!(orbit.angular_speed > 0.0);
        }
    }

    #[test]
    fn earth_completes_one_orbit_in_five_minutes() {
        let mut game = Game::new();
        let initial = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap().position;
        let earth_orbit = game.orbits.iter().find(|orbit| orbit.body_kind == PlanetKind::Earth).unwrap();
        let period = std::f32::consts::TAU / earth_orbit.angular_speed;
        assert!((period - 300.0).abs() < 0.01);
        game.update_orbits(300.0);
        let after = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap().position;
        assert!(distance(initial, after) < 1.0);
    }

    #[test]
    fn planetary_periods_follow_kepler_ratios() {
        let game = Game::new();
        let period = |kind| {
            let orbit = game.orbits.iter().find(|orbit| orbit.body_kind == kind).unwrap();
            std::f32::consts::TAU / orbit.angular_speed
        };
        assert!(period(PlanetKind::Mercury) < period(PlanetKind::Venus));
        assert!(period(PlanetKind::Venus) < period(PlanetKind::Earth));
        assert!(period(PlanetKind::Earth) < period(PlanetKind::Mars));
        assert!(period(PlanetKind::Mars) < period(PlanetKind::Jupiter));
        assert!(period(PlanetKind::Jupiter) < period(PlanetKind::Saturn));
        assert!(period(PlanetKind::Saturn) < period(PlanetKind::Uranus));
        assert!(period(PlanetKind::Uranus) < period(PlanetKind::Neptune));
    }

    #[test]
    fn solar_orbital_distances_keep_approximate_au_ratios() {
        let game = Game::new();
        let sun = game.bodies.iter().find(|body| body.kind == PlanetKind::Sun).unwrap().position;
        for (kind, expected_au) in [
            (PlanetKind::Mercury, 0.387_f32), (PlanetKind::Venus, 0.723),
            (PlanetKind::Earth, 1.0), (PlanetKind::Mars, 1.524),
            (PlanetKind::Jupiter, 5.203), (PlanetKind::Saturn, 9.537),
            (PlanetKind::Uranus, 19.191), (PlanetKind::Neptune, 30.069),
        ] {
            let body = game.bodies.iter().find(|body| body.kind == kind).unwrap();
            let measured_au = distance(sun, body.position) / AU_SCALE;
            assert!((measured_au - expected_au).abs() < 0.02, "{kind:?}: {measured_au}");
        }
    }

    #[test]
    fn asteroid_belt_spans_a_full_annular_orbit() {
        assert!(ASTEROID_BELT_INNER_AU > 2.0);
        assert!(ASTEROID_BELT_OUTER_AU > ASTEROID_BELT_INNER_AU);
        let game = Game::new();
        let sun = game.bodies.iter().find(|body| body.kind == PlanetKind::Sun).unwrap().position;
        let belt = game.bodies.iter().find(|body| body.kind == PlanetKind::AsteroidBelt).unwrap();
        let radius_au = distance(sun, belt.position) / AU_SCALE;
        assert!((radius_au - 2.7).abs() < 0.01);
    }

    #[test]
    fn wormhole_remains_next_to_saturn_as_saturn_orbits() {
        let mut game = Game::new();
        let offset_before = {
            let saturn = game.bodies.iter().find(|body| body.kind == PlanetKind::Saturn).unwrap();
            let wormhole = game.bodies.iter().find(|body| body.kind == PlanetKind::Wormhole).unwrap();
            wormhole.position - saturn.position
        };
        assert!((length(offset_before) - (18_000.0_f32.powi(2) + 3_900.0_f32.powi(2)).sqrt()).abs() < 20.0);

        for _ in 0..240 {
            game.update(0.05, InputState::default());
        }

        let saturn = game.bodies.iter().find(|body| body.kind == PlanetKind::Saturn).unwrap();
        let wormhole = game.bodies.iter().find(|body| body.kind == PlanetKind::Wormhole).unwrap();
        let offset_after = wormhole.position - saturn.position;
        assert!(distance(offset_before, offset_after) < 1.0);
    }

    #[test]
    fn celestial_bodies_use_fifteen_times_base_scale() {
        let game = Game::new();
        assert_eq!(PLANET_SCALE, 15.0);
        let radius = |kind| game.bodies.iter().find(|body| body.kind == kind).unwrap().radius;
        assert_eq!(radius(PlanetKind::Earth), 180.0 * 15.0);
        assert_eq!(radius(PlanetKind::Sun), 300.0 * 15.0);
        assert_eq!(radius(PlanetKind::Wormhole), 38.0 * 15.0);
        assert_eq!(radius(PlanetKind::Gargantua), GARGANTUA_RADIUS);
        assert!((GARGANTUA_MILLER_ORBIT - GARGANTUA_DISK_RADIUS - GARGANTUA_DISK_ORBIT_GAP).abs() < 0.001);
    }

    #[test]
    fn gargantua_planet_orbits_clear_the_central_model_and_disk_volume() {
        let game = Game::new();
        let miller = game.orbits.iter().find(|orbit| orbit.body_kind == PlanetKind::MillerWater).unwrap();
        let mann = game.orbits.iter().find(|orbit| orbit.body_kind == PlanetKind::MannIce).unwrap();
        let edmund = game.orbits.iter().find(|orbit| orbit.body_kind == PlanetKind::EdmundHabitable).unwrap();
        assert!(miller.radius > GARGANTUA_RADIUS * 2.0);
        assert!((miller.radius - GARGANTUA_DISK_RADIUS - GARGANTUA_DISK_ORBIT_GAP).abs() < 0.001);
        assert!(miller.radius < mann.radius && mann.radius < edmund.radius);
        assert!(miller.radius > 80_000.0);
        assert!(edmund.radius > 200_000.0);
    }

    #[test]
    fn wormhole_exit_is_outside_the_outer_gargantua_orbit() {
        let exit_distance = distance(wormhole_exit(), gargantua_position());
        assert!(exit_distance > GARGANTUA_EDMUND_ORBIT);
        assert!(exit_distance - GARGANTUA_EDMUND_ORBIT > 30_000.0);
    }

    #[test]
    fn saturn_wormhole_is_outside_the_enlarged_ring_system() {
        let game = Game::new();
        let saturn = game.bodies.iter().find(|body| body.kind == PlanetKind::Saturn).unwrap();
        let wormhole = game.bodies.iter().find(|body| body.kind == PlanetKind::Wormhole).unwrap();
        let center_gap = distance(saturn.position, wormhole.position);
        let conservative_combined_extent = saturn.radius * 2.5 + wormhole.radius * 3.0;
        assert!(center_gap > conservative_combined_extent);
    }

    #[test]
    fn inner_planets_and_sun_keep_surface_clearance() {
        let game = Game::new();
        let body = |kind| game.bodies.iter().find(|item| item.kind == kind).unwrap();
        for planet_kind in [PlanetKind::Mercury, PlanetKind::Venus] {
            let planet = body(planet_kind);
            assert!(distance(body(PlanetKind::Sun).position, planet.position)
                > body(PlanetKind::Sun).radius + planet.radius);
        }
    }

    #[test]
    fn regular_boost_reaches_but_never_exceeds_one_thousand() {
        let mut game = Game::new();
        let input = InputState {
            forward: true,
            boost: true,
            superboost: false,
            ..InputState::default()
        };
        for _ in 0..160 {
            game.update(0.05, input);
        }
        assert!(game.boost_active);
        assert!(game.speed > 999.0, "speed was {}", game.speed);
        assert!(game.speed <= BOOST_MAX_SPEED);
        assert!(length(game.velocity) <= BOOST_MAX_SPEED + 0.1);
    }

    #[test]
    fn superboost_reaches_but_never_exceeds_two_thousand_five_hundred() {
        let mut game = Game::new();
        let input = InputState {
            forward: true,
            boost: true,
            superboost: true,
            ..InputState::default()
        };
        for _ in 0..160 {
            game.update(0.05, input);
        }
        assert!(game.superboost_active);
        assert!(game.speed > 2_499.0, "speed was {}", game.speed);
        assert!(game.speed <= SUPERBOOST_MAX_SPEED);
        assert!(length(game.velocity) <= SUPERBOOST_MAX_SPEED + 0.1);

        let speed_before_release = length(game.velocity);
        game.update(0.05, InputState::default());
        let speed_after_release = length(game.velocity);
        assert!(speed_after_release > speed_before_release * 0.90,
            "releasing superboost should decelerate smoothly, not snap: {speed_before_release} -> {speed_after_release}");
    }

    #[test]
    fn pressing_b_without_shift_does_not_enable_superboost() {
        let mut game = Game::new();
        let input = InputState {
            forward: true,
            boost: false,
            superboost: true,
            ..InputState::default()
        };
        game.update(0.05, input);
        assert!(!game.superboost_active);
        assert!(game.speed < 100.0);
    }

    #[test]
    fn gravity_changes_the_ship_velocity_toward_a_nearby_planet() {
        let mut game = Game::new();
        let earth = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap();
        game.position = earth.position + vec3(0.0, 0.0, earth.radius * 2.0);
        let gravity = game.gravity_acceleration();
        assert!(gravity.z < 0.0, "gravity should pull the ship toward Earth: {gravity:?}");
        game.update(0.05, InputState::default());
        assert!(game.velocity.z < -46.0);
    }

    #[test]
    fn predictive_avoidance_keeps_the_ship_outside_earths_safe_shell() {
        let mut game = Game::new();
        let earth = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap();
        let center = earth.position;
        let safe_radius = Game::collision_radius(earth);
        game.position = center + vec3(0.0, 0.0, safe_radius + 900.0);
        game.velocity = vec3(0.0, 0.0, -2_500.0);
        game.speed = 2_500.0;
        for _ in 0..40 {
            game.update(0.05, InputState::default());
            let earth_now = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap();
            assert!(distance(game.position, earth_now.position) >= safe_radius - 0.5,
                "ship entered Earth's no-fly volume");
        }
        assert!(game.position.x.abs() > 1.0 || game.position.y.abs() > 1.0,
            "avoidance should bend the trajectory around the planet");
    }

    #[test]
    fn orbit_assist_toggles_for_a_nearby_body() {
        let mut game = Game::new();
        let earth_index = game.bodies.iter().position(|body| body.kind == PlanetKind::Earth).unwrap();
        let earth = &game.bodies[earth_index];
        game.position = earth.position + vec3(0.0, 0.0, earth.radius * 2.5);
        game.velocity = vec3(0.0, 0.0, -80.0);
        game.toggle_orbit_assist();
        assert_eq!(game.orbit_target, Some(earth_index));
        game.toggle_orbit_assist();
        assert_eq!(game.orbit_target, None);
    }

    #[test]
    fn place_opens_planet_side_and_both_system_overviews() {
        let mut game = Game::new();
        game.place_named("earth").unwrap();
        let earth = game.bodies.iter().find(|body| body.kind == PlanetKind::Earth).unwrap();
        assert!(game.position.z > earth.position.z, "Earth placement should be on the Sun-facing side");
        assert!(game.forward().z < -0.99, "ship should face the selected body");

        game.place_named("vista superior galaxia 2").unwrap();
        assert!(game.is_overview());
        assert!(game.crossed_wormhole);
        let (eye, target) = game.camera();
        let gargantua = game.bodies.iter().find(|body| body.kind == PlanetKind::Gargantua).unwrap();
        assert!(distance(target, gargantua.position) < 0.01);
        assert!(eye.y > target.y);

        game.place_named("galaxy1-side").unwrap();
        assert!(game.is_overview());
        assert!(!game.crossed_wormhole);
        let (_, target) = game.camera();
        let sun = game.bodies.iter().find(|body| body.kind == PlanetKind::Sun).unwrap();
        assert!(distance(target, sun.position) < 0.01);
    }

    #[test]
    fn every_photo_and_place_destination_resolves() {
        let mut game = Game::new();
        for target in [
            "mercury", "venus", "earth", "moon", "mars", "jupiter", "saturn",
            "uranus", "neptune", "miller", "mann", "edmund", "sun", "wormhole",
            "gargantua", "endurance", "galaxy1-top", "galaxy1-side", "galaxy2-top", "galaxy2-side",
        ] {
            game.place_named(target).unwrap_or_else(|error| panic!("{target}: {error}"));
        }
    }

    #[test]
    fn miller_sun_facing_spawn_stays_outside_gargantua_safe_shell() {
        let mut game = Game::new();
        game.place_named("miller").unwrap();
        let gargantua = game.bodies.iter().find(|body| body.kind == PlanetKind::Gargantua).unwrap();
        let miller = game.bodies.iter().find(|body| body.kind == PlanetKind::MillerWater).unwrap();
        assert!(distance(game.position, gargantua.position) > Game::collision_radius(gargantua) + 500.0);
        assert!(distance(game.position, miller.position) > Game::collision_radius(miller));
        assert!(dot(normalize(gargantua.position - miller.position), normalize(game.position - miller.position)) > 0.99,
            "the ship should spawn on the side of Miller facing Gargantua");
    }

    #[test]
    fn acceleration_builds_up_progressively_instead_of_jumping_to_cruise_speed() {
        let mut game = Game::new();
        game.position = vec3(500_000.0, 500_000.0, 500_000.0);
        game.velocity = vec3(0.0, 0.0, 0.0);
        game.speed = 0.0;
        let thrust = InputState { forward: true, ..InputState::default() };
        game.update(0.05, thrust);
        let first_step = game.speed;
        for _ in 0..9 { game.update(0.05, thrust); }
        let half_second = game.speed;
        for _ in 0..30 { game.update(0.05, thrust); }
        let two_seconds = game.speed;
        assert!(first_step > 0.0);
        assert!(first_step < half_second && half_second < two_seconds,
            "acceleration should build gradually: {first_step}, {half_second}, {two_seconds}");
        assert!(two_seconds < 215.0);
    }

    #[test]
    fn braking_reduces_speed_progressively_without_an_instant_stop() {
        let mut game = Game::new();
        game.position = vec3(500_000.0, 500_000.0, 500_000.0);
        game.velocity = vec3(0.0, 0.0, -800.0);
        game.speed = 800.0;
        let brake = InputState { back: true, ..InputState::default() };
        game.update(0.05, brake);
        let after_one_step = game.speed;
        for _ in 0..19 { game.update(0.05, brake); }
        let after_one_second = game.speed;
        assert!(after_one_step < 800.0 && after_one_step > after_one_second);
        assert!(after_one_second > 0.0);
    }

    #[test]
    fn steering_ramps_up_and_decays_smoothly() {
        let mut game = Game::new();
        game.position = vec3(500_000.0, 500_000.0, 500_000.0);
        game.velocity = vec3(0.0, 0.0, -58.0);
        game.speed = 58.0;
        let turn = InputState { left: true, ..InputState::default() };
        game.update(0.05, turn);
        let first_yaw = game.yaw.abs();
        for _ in 0..19 { game.update(0.05, turn); }
        let sustained_yaw = game.yaw.abs();
        game.update(0.05, InputState::default());
        assert!(first_yaw > 0.0 && first_yaw < sustained_yaw);
        assert!(game.yaw_rate.abs() < 1.1);
    }

    #[test]
    fn flight_controls_are_stable_across_frame_rates() {
        fn simulate(frame_dt: f32, frames: usize) -> Game {
            let mut game = Game::new();
            game.position = vec3(500_000.0, 500_000.0, 500_000.0);
            game.velocity = vec3(0.0, 0.0, -58.0);
            game.speed = 58.0;
            let input = InputState { forward: true, left: true, pitch_up: true, ..InputState::default() };
            for _ in 0..frames {
                game.update(frame_dt, input);
            }
            game
        }
        let at_10 = simulate(0.1, 30);
        let at_30 = simulate(1.0 / 30.0, 90);
        let at_60 = simulate(1.0 / 60.0, 180);
        let at_144 = simulate(1.0 / 144.0, 432);
        assert!(distance(at_10.position, at_60.position) < 30.0);
        assert!(distance(at_30.position, at_60.position) < 30.0);
        assert!(distance(at_60.position, at_144.position) < 30.0);
        assert!((at_10.speed - at_144.speed).abs() < 2.0);
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
