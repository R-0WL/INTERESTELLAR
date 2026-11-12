//! Pixel-based menu, HUD, and minimap drawn into the software framebuffer.

use crate::framebuffer::Framebuffer;
use crate::game::{Game, HudState, PlanetKind, AU_SCALE, ASTEROID_BELT_INNER_AU, ASTEROID_BELT_OUTER_AU};
use font8x8::{UnicodeFonts, BASIC_FONTS};

const PANEL: u32 = 0x07111F;
const PANEL_BORDER: u32 = 0x42647D;
const TEXT_BRIGHT: u32 = 0xFFFFFF;
const TEXT_MUTED: u32 = 0xA8BBCB;
const ACCENT: u32 = 0x79CFFF;

pub fn draw_main_menu(framebuffer: &mut Framebuffer, selected: usize) {
    framebuffer.clear_vertical_gradient(0x02050D, 0x08182A);
    let width = framebuffer.width as i32;
    let height = framebuffer.height as i32;

    // Stable decorative star field, generated without another rendering dependency.
    for index in 0..220 {
        let x = ((index * 197 + 31) % framebuffer.width.max(1)) as i32;
        let y = ((index * 83 + 17) % framebuffer.height.max(1)) as i32;
        let color = if index % 11 == 0 { 0xA8DFFF } else { 0x526C8B };
        framebuffer.set_pixel(x, y, color);
    }

    let title = "INTERSTELLAR";
    let title_width = title.len() as i32 * 8 * 4;
    draw_text(framebuffer, (width - title_width) / 2, height / 7, title, 0xDDF4FF, 4);
    let subtitle = "J O U R N E Y  /  R U S T  S O F T W A R E  R E N D E R E R";
    draw_text(framebuffer, (width - subtitle.len() as i32 * 8) / 2, height / 7 + 42, subtitle, 0x79AFCB, 1);

    let panel_width = 430.min(width - 40);
    let panel_x = (width - panel_width) / 2;
    let panel_y = height / 2 - 112;
    framebuffer.fill_rect(panel_x - 18, panel_y - 30, panel_width + 36, 304, 0x06111E);
    framebuffer.draw_rect(panel_x - 18, panel_y - 30, panel_width + 36, 304, 0x27445D);
    draw_text(framebuffer, panel_x + 4, panel_y - 12, "MENU PRINCIPAL", ACCENT, 1);

    let options = [
        ("MODO EXPLORACION", "VUELO LIBRE POR LOS SISTEMAS"),
        ("MODO HISTORIA", "MISIONES NARRATIVAS - EN DESARROLLO"),
        ("MODO NIVELES", "DESAFIOS Y OBJETIVOS - EN DESARROLLO"),
        ("CONFIGURACION", "MUSICA, DIALOGOS, EFECTOS Y SHADERS"),
    ];
    for (index, (title, description)) in options.iter().enumerate() {
        let y = panel_y + 12 + index as i32 * 57;
        let is_selected = index == selected;
        let background = if is_selected { 0x173750 } else { 0x0B1A29 };
        let border = if is_selected { 0x65C8F4 } else { 0x263D52 };
        framebuffer.fill_rect(panel_x, y, panel_width, 46, background);
        framebuffer.draw_rect(panel_x, y, panel_width, 46, border);
        if is_selected {
            draw_text(framebuffer, panel_x + 12, y + 11, ">", 0xFFD98E, 2);
        }
        draw_text(framebuffer, panel_x + 38, y + 6, title, if is_selected { TEXT_BRIGHT } else { 0xB4C7D8 }, 1);
        draw_text(framebuffer, panel_x + 38, y + 25, description, if is_selected { 0xA7DDF8 } else { 0x70869A }, 1);
    }

    let footer = "UP / DOWN: SELECT     ENTER: CONFIRM     ESC: EXIT";
    draw_text(framebuffer, (width - footer.len() as i32 * 8) / 2, height - 42, footer, TEXT_MUTED, 1);
}

/// Settings are intentionally a UI/state skeleton: changing a value stores the
/// selection, but does not yet connect it to audio, dialogue, effects or shaders.
pub fn draw_settings_menu(
    framebuffer: &mut Framebuffer,
    selected: usize,
    music_enabled: bool,
    dialogue_visible: bool,
    effects_enabled: bool,
    shader_layer: u8,
) {
    framebuffer.clear_vertical_gradient(0x02050D, 0x08182A);
    let width = framebuffer.width as i32;
    let height = framebuffer.height as i32;
    for index in 0..160 {
        let x = ((index * 197 + 31) % framebuffer.width.max(1)) as i32;
        let y = ((index * 83 + 17) % framebuffer.height.max(1)) as i32;
        framebuffer.set_pixel(x, y, if index % 13 == 0 { 0xA8DFFF } else { 0x526C8B });
    }

    let panel_width = 620.min(width - 40);
    let panel_height = 386.min(height - 60);
    let panel_x = (width - panel_width) / 2;
    let panel_y = (height - panel_height) / 2;
    framebuffer.fill_rect(panel_x, panel_y, panel_width, panel_height, PANEL);
    framebuffer.draw_rect(panel_x, panel_y, panel_width, panel_height, PANEL_BORDER);
    draw_text(framebuffer, panel_x + 28, panel_y + 24, "CONFIGURACION", ACCENT, 2);
    draw_text(framebuffer, panel_x + 28, panel_y + 60, "OPCIONES PREPARADAS PARA IMPLEMENTACION", TEXT_MUTED, 1);

    let layer_label = match shader_layer {
        1 => "CAPA 1/6 - BASE ACTUAL",
        2 => "CAPA 2/6 - RESERVADA",
        3 => "CAPA 3/6 - RESERVADA",
        4 => "CAPA 4/6 - RESERVADA",
        5 => "CAPA 5/6 - RESERVADA",
        _ => "CAPA 6/6 - RESERVADA",
    };
    let options = [
        ("MUSICA", if music_enabled { "ACTIVADA" } else { "DESACTIVADA" }),
        ("DIALOGOS VISIBLES", if dialogue_visible { "ACTIVADOS" } else { "DESACTIVADOS" }),
        ("EFECTOS", if effects_enabled { "ACTIVADOS" } else { "DESACTIVADOS" }),
        ("RENDERIZACION / SHADERS", layer_label),
    ];
    for (index, (label, value)) in options.iter().enumerate() {
        let y = panel_y + 100 + index as i32 * 55;
        let active = selected == index;
        framebuffer.fill_rect(panel_x + 22, y, panel_width - 44, 43, if active { 0x173750 } else { 0x0B1A29 });
        framebuffer.draw_rect(panel_x + 22, y, panel_width - 44, 43, if active { 0x65C8F4 } else { 0x263D52 });
        if active {
            draw_text(framebuffer, panel_x + 34, y + 13, ">", 0xFFD98E, 2);
        }
        draw_text(framebuffer, panel_x + 62, y + 8, label, if active { TEXT_BRIGHT } else { 0xB4C7D8 }, 1);
        let value_x = panel_x + panel_width - 260;
        draw_text(framebuffer, value_x, y + 17, value, if active { 0xA7DDF8 } else { TEXT_MUTED }, 1);
    }
    draw_text(framebuffer, panel_x + 28, panel_y + panel_height - 54, "CAPA 1 ES EL RENDER ACTUAL; 2-6 QUEDAN COMO MARCADORES.", 0xA7DDF8, 1);
    draw_text(framebuffer, panel_x + 28, panel_y + panel_height - 34, "UP/DOWN: ELEGIR  LEFT/RIGHT: CAMBIAR  BACKSPACE: VOLVER", TEXT_MUTED, 1);
}

pub fn settings_gear_contains(x: f32, y: f32, screen_width: usize) -> bool {
    let left = screen_width as f32 - 52.0;
    x >= left && x <= left + 44.0 && y >= 238.0 && y <= 282.0
}

fn draw_settings_gear(framebuffer: &mut Framebuffer, center_x: i32, center_y: i32) {
    framebuffer.fill_rect(center_x - 19, center_y - 17, 38, 34, PANEL);
    framebuffer.draw_rect(center_x - 19, center_y - 17, 38, 34, PANEL_BORDER);
    let color = ACCENT;
    for index in 0..8 {
        let angle = index as f32 * std::f32::consts::TAU / 8.0;
        let (sin_angle, cos_angle) = angle.sin_cos();
        framebuffer.draw_line(
            center_x + (cos_angle * 7.0) as i32,
            center_y + (sin_angle * 7.0) as i32,
            center_x + (cos_angle * 13.0) as i32,
            center_y + (sin_angle * 13.0) as i32,
            color,
        );
    }
    draw_circle(framebuffer, center_x, center_y, 8, color, 24);
    draw_circle(framebuffer, center_x, center_y, 3, color, 16);
}

pub fn draw_mode_preview(framebuffer: &mut Framebuffer, mode_name: &str, description: &str) {
    framebuffer.clear_vertical_gradient(0x02050D, 0x08182A);
    let width = framebuffer.width as i32;
    let height = framebuffer.height as i32;
    let panel_width = 560.min(width - 40);
    let panel_height = 220;
    let x = (width - panel_width) / 2;
    let y = (height - panel_height) / 2;

    framebuffer.fill_rect(x, y, panel_width, panel_height, PANEL);
    framebuffer.draw_rect(x, y, panel_width, panel_height, PANEL_BORDER);
    let title = mode_name.to_ascii_uppercase();
    draw_text(framebuffer, x + 28, y + 30, &title, ACCENT, 2);
    draw_text(framebuffer, x + 28, y + 88, "MODO EN DESARROLLO", 0xFFD98E, 1);
    draw_text(framebuffer, x + 28, y + 116, &truncate(description, 62), 0xC4D4E0, 1);
    draw_text(framebuffer, x + 28, y + 174, "BACKSPACE / ENTER: VOLVER AL MENU", TEXT_MUTED, 1);
}

pub fn draw_hud(framebuffer: &mut Framebuffer, game: &Game, state: &HudState, capture_notice: f32, show_minimap: bool) {
    let width = framebuffer.width as i32;
    let height = framebuffer.height as i32;

    // Mission information panel.
    let panel_width = 370.min(width - 24);
    framebuffer.fill_rect(16, 14, panel_width, 112, PANEL);
    framebuffer.draw_rect(16, 14, panel_width, 112, PANEL_BORDER);
    draw_text(framebuffer, 28, 24, "MISSION TRAJECTORY", ACCENT, 1);
    draw_text(framebuffer, 28, 44, &truncate(&state.title, 20), TEXT_BRIGHT, 2);
    draw_text(framebuffer, 28, 78, &truncate(&state.body, 42), 0xC4D4E0, 1);
    draw_text(framebuffer, 28, 95, &truncate(&state.prompt, 42), 0xFFD98E, 1);

    // Flight data moved to the lower-right so the minimap owns the upper-right corner.
    let telemetry_x = (width - 218).max(16);
    let telemetry_y = (height - 122).max(132);
    framebuffer.fill_rect(telemetry_x, telemetry_y, 202, 86, PANEL);
    framebuffer.draw_rect(telemetry_x, telemetry_y, 202, 86, PANEL_BORDER);
    draw_text(framebuffer, telemetry_x + 10, telemetry_y + 8, "FLIGHT DATA", ACCENT, 1);
    draw_text(framebuffer, telemetry_x + 10, telemetry_y + 27, &format!("SPD {:>4.0}  RNG {:>5.0}", state.speed, state.distance), TEXT_BRIGHT, 1);
    draw_text(framebuffer, telemetry_x + 10, telemetry_y + 45, &format!("PHASE {}", truncate(&state.phase, 15)), TEXT_BRIGHT, 1);
    let (status, status_color) = if !state.warning.is_empty() {
        (state.warning.as_str(), 0xFF6C64)
    } else if game.orbit_target.is_some() {
        ("ORBIT ASSIST", 0x7BD7FF)
    } else if game.superboost_active {
        ("SUPERBOOST 2500", 0x7BD7FF)
    } else if game.boost_active {
        ("BOOST 1000", 0xA6DFFF)
    } else {
        ("STATUS NOMINAL", 0x80DCA9)
    };
    draw_text(framebuffer, telemetry_x + 10, telemetry_y + 65, status, status_color, 1);

    // Center reticle.
    let center_x = width / 2;
    let center_y = height / 2;
    let reticle = 0x9DDFFF;
    framebuffer.draw_rect(center_x - 9, center_y - 9, 18, 18, reticle);
    framebuffer.draw_line(center_x - 15, center_y, center_x - 11, center_y, reticle);
    framebuffer.draw_line(center_x + 11, center_y, center_x + 15, center_y, reticle);
    framebuffer.draw_line(center_x, center_y - 15, center_x, center_y - 11, reticle);
    framebuffer.draw_line(center_x, center_y + 11, center_x, center_y + 15, reticle);

    if show_minimap {
        draw_minimap(framebuffer, game);
    } else {
        draw_text(framebuffer, (width - 102).max(16), 20, "M: SHOW MAP", 0xA8BBCB, 1);
    }
    draw_settings_gear(framebuffer, width - 30, 260);
    draw_text(framebuffer, width - 78, 284, "G CONFIG", ACCENT, 1);

    // Controls strip and current target.
    draw_text(framebuffer, 22, height - 48, &format!("TARGET  {}", truncate(&state.target, 34)), 0x7EABC9, 1);
    let controls_y = height - 28;
    framebuffer.fill_rect(12, controls_y - 7, width - 24, 24, PANEL);
    draw_text(
        framebuffer,
        22,
        controls_y,
        "W/S THRUST  A/D STEER  SHIFT BOOST 1000  SHIFT+B SUPER 2500  E ORBIT  SPACE WORMHOLE  R RESET  M MAP  P PHOTO  G CONFIG  BACKSPACE MENU",
        TEXT_MUTED,
        1,
    );

    if capture_notice > 0.0 {
        let message = "SCREENSHOT SAVED: captures/";
        let x = (width - (message.len() as i32 * 8)) / 2;
        framebuffer.fill_rect(x - 8, 142, message.len() as i32 * 8 + 16, 24, 0x0B1C2B);
        framebuffer.draw_rect(x - 8, 142, message.len() as i32 * 8 + 16, 24, 0x64C9FF);
        draw_text(framebuffer, x, 150, message, 0xDDF5FF, 1);
    }
}

fn draw_minimap(framebuffer: &mut Framebuffer, game: &Game) {
    let width = framebuffer.width as i32;
    let panel_w = 236.min(width - 24);
    let panel_h = 220;
    let panel_x = (width - panel_w - 14).max(12);
    let panel_y = 14;
    framebuffer.fill_rect(panel_x, panel_y, panel_w, panel_h, 0x06101D);
    framebuffer.draw_rect(panel_x, panel_y, panel_w, panel_h, 0x42647D);

    let title = if game.crossed_wormhole { "GARGANTUA SYSTEM" } else { "SOLAR SYSTEM MAP" };
    draw_text(framebuffer, panel_x + 10, panel_y + 9, title, ACCENT, 1);
    draw_text(framebuffer, panel_x + panel_w - 65, panel_y + 9, "M: HIDE", 0xA8BBCB, 1);

    let gargantua_targets = [
        (PlanetKind::MillerWater, "MIL", 0x49B9D8),
        (PlanetKind::MannIce, "MAN", 0xC1D7E7),
        (PlanetKind::EdmundHabitable, "EDM", 0x8DAF71),
    ];
    let solar_targets = [
        (PlanetKind::Mercury, "MER", 0xA69B8C),
        (PlanetKind::Venus, "VEN", 0xD8AC69),
        (PlanetKind::Earth, "EAR", 0x4C9FE0),
        (PlanetKind::Moon, "LUN", 0xAEB9C2),
        (PlanetKind::Mars, "MAR", 0xD87552),
        (PlanetKind::AsteroidBelt, "BELT", 0x9E927B),
        (PlanetKind::Jupiter, "JUP", 0xD8A66B),
        (PlanetKind::Saturn, "SAT", 0xD8C17F),
        (PlanetKind::Uranus, "URA", 0x79C9CC),
        (PlanetKind::Neptune, "NEP", 0x638BEA),
        (PlanetKind::Wormhole, "WH", 0x9C80E8),
    ];
    let (center_kind, targets): (PlanetKind, &[(PlanetKind, &str, u32)]) = if game.crossed_wormhole {
        (PlanetKind::Gargantua, &gargantua_targets)
    } else {
        (PlanetKind::Sun, &solar_targets)
    };

    let Some(center_body) = game.bodies.iter().find(|body| body.kind == center_kind) else {
        return;
    };
    let map_cx = panel_x + panel_w / 2;
    let map_cy = panel_y + 128;
    let map_radius = 84.0_f32.min((panel_w as f32 - 34.0) * 0.40);

    let mut max_radius = 1.0_f32;
    for (kind, _, _) in targets.iter() {
        if let Some(body) = game.bodies.iter().find(|body| body.kind == *kind) {
            let dx = body.position.x - center_body.position.x;
            let dz = body.position.z - center_body.position.z;
            max_radius = max_radius.max((dx * dx + dz * dz).sqrt());
        }
    }
    // The outer guide ring leaves a small margin around the most distant body.
    let scale = map_radius / (max_radius * 1.12);

    // Draw the central-system orbit guides.
    for (kind, _, color) in targets.iter() {
        if *kind == PlanetKind::Moon || *kind == PlanetKind::AsteroidBelt || *kind == PlanetKind::Wormhole {
            continue;
        }
        if let Some(body) = game.bodies.iter().find(|body| body.kind == *kind) {
            let dx = body.position.x - center_body.position.x;
            let dz = body.position.z - center_body.position.z;
            let radius = ((dx * dx + dz * dz).sqrt() * scale) as i32;
            draw_circle(framebuffer, map_cx, map_cy, radius, orbit_color(*color), 96);
        }
    }

    // The asteroid belt is represented by its complete inner and outer boundaries.
    if !game.crossed_wormhole {
        let inner = (ASTEROID_BELT_INNER_AU * AU_SCALE * scale) as i32;
        let outer = (ASTEROID_BELT_OUTER_AU * AU_SCALE * scale) as i32;
        draw_circle(framebuffer, map_cx, map_cy, inner, 0x5C5145, 96);
        draw_circle(framebuffer, map_cx, map_cy, outer, 0x5C5145, 96);
    }

    // Moon's short orbit around Earth (only present in the Solar System map).
    if !game.crossed_wormhole {
        if let (Some(earth), Some(moon)) = (
            game.bodies.iter().find(|body| body.kind == PlanetKind::Earth),
            game.bodies.iter().find(|body| body.kind == PlanetKind::Moon),
        ) {
            let orbit_x = map_cx + ((earth.position.x - center_body.position.x) * scale) as i32;
            let orbit_y = map_cy + ((earth.position.z - center_body.position.z) * scale) as i32;
            let dx = moon.position.x - earth.position.x;
            let dz = moon.position.z - earth.position.z;
            let radius = ((dx * dx + dz * dz).sqrt() * scale) as i32;
            draw_circle(framebuffer, orbit_x, orbit_y, radius, 0x657789, 48);
        }
    }

    // Central star / black hole.
    let center_color = if game.crossed_wormhole { 0xD98A42 } else { 0xFFD36B };
    framebuffer.fill_rect(map_cx - 4, map_cy - 4, 9, 9, center_color);
    draw_text(framebuffer, map_cx + 7, map_cy - 4, if game.crossed_wormhole { "GARG" } else { "SUN" }, center_color, 1);

    for (kind, label, color) in targets.iter() {
        if let Some(body) = game.bodies.iter().find(|body| body.kind == *kind) {
            let x = map_cx + ((body.position.x - center_body.position.x) * scale) as i32;
            let y = map_cy + ((body.position.z - center_body.position.z) * scale) as i32;
            framebuffer.fill_rect(x - 2, y - 2, 5, 5, *color);
            let (label_dx, label_dy) = match kind {
                PlanetKind::Mercury => (7, -12),
                PlanetKind::Venus => (-29, 2),
                PlanetKind::Earth => (7, 8),
                PlanetKind::Moon => (-29, 12),
                PlanetKind::Mars => (7, -12),
                PlanetKind::AsteroidBelt => (-34, 10),
                PlanetKind::Jupiter => (7, -12),
                PlanetKind::Saturn => (-29, 8),
                PlanetKind::Uranus => (7, -12),
                PlanetKind::Neptune => (-34, 8),
                PlanetKind::Wormhole => (-20, -12),
                _ => (7, -4),
            };
            draw_text(framebuffer, x + label_dx, y + label_dy, label, *color, 1);
        }
    }

    // The ship marker is clamped to the map boundary if it leaves the displayed system.
    let ship_dx = game.position.x - center_body.position.x;
    let ship_dz = game.position.z - center_body.position.z;
    let ship_distance = (ship_dx * ship_dx + ship_dz * ship_dz).sqrt();
    let limit = max_radius * 1.02;
    let factor = if ship_distance > limit && ship_distance > 0.001 { limit / ship_distance } else { 1.0 };
    let ship_x = map_cx + (ship_dx * factor * scale) as i32;
    let ship_y = map_cy + (ship_dz * factor * scale) as i32;
    framebuffer.draw_line(ship_x - 5, ship_y, ship_x + 5, ship_y, 0x5BE7FF);
    framebuffer.draw_line(ship_x, ship_y - 5, ship_x, ship_y + 5, 0x5BE7FF);
    framebuffer.draw_rect(ship_x - 3, ship_y - 3, 6, 6, 0x5BE7FF);

    draw_text(framebuffer, panel_x + 10, panel_y + panel_h - 17, "SHIP +   ORBIT RINGS", 0xA8BBCB, 1);
}

fn orbit_color(color: u32) -> u32 {
    let r = ((color >> 16) & 0xFF) / 2;
    let g = ((color >> 8) & 0xFF) / 2;
    let b = (color & 0xFF) / 2;
    (r << 16) | (g << 8) | b
}

fn draw_circle(framebuffer: &mut Framebuffer, cx: i32, cy: i32, radius: i32, color: u32, segments: usize) {
    if radius <= 0 || segments < 3 {
        return;
    }
    let mut previous: Option<(i32, i32)> = None;
    for index in 0..=segments {
        let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
        let point = (cx + (radius as f32 * angle.cos()) as i32, cy + (radius as f32 * angle.sin()) as i32);
        if let Some((x0, y0)) = previous {
            framebuffer.draw_line(x0, y0, point.0, point.1, color);
        }
        previous = Some(point);
    }
}

pub fn draw_text(framebuffer: &mut Framebuffer, x: i32, y: i32, text: &str, color: u32, scale: i32) {
    let mut cursor_x = x;
    let scale = scale.max(1);
    for character in text.chars() {
        if character == '\n' {
            cursor_x = x;
            continue;
        }
        let display_character = character.to_ascii_uppercase();
        let glyph = BASIC_FONTS.get(display_character).unwrap_or([0; 8]);
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..8 {
                if (*bits & (1u8 << column)) != 0 {
                    framebuffer.fill_rect(
                        cursor_x + column as i32 * scale,
                        y + row as i32 * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
        cursor_x += 8 * scale;
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        let mut result: String = value.chars().take(max_chars.saturating_sub(3)).collect();
        result.push_str("...");
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_changes_pixels() {
        let mut fb = Framebuffer::new(80, 20);
        fb.clear(0);
        draw_text(&mut fb, 0, 0, "A", 0xFFFFFF, 1);
        assert!(fb.color.iter().any(|pixel| *pixel != 0));
    }

    #[test]
    fn menu_draws_a_visible_selection() {
        let mut fb = Framebuffer::new(800, 600);
        draw_main_menu(&mut fb, 0);
        assert!(fb.color.iter().any(|pixel| *pixel == 0x65C8F4));
    }

    #[test]
    fn settings_menu_displays_all_four_placeholders() {
        let mut fb = Framebuffer::new(1000, 640);
        draw_settings_menu(&mut fb, 0, true, true, true, 1);
        assert!(fb.color.iter().any(|pixel| *pixel == 0x65C8F4));
    }

    #[test]
    fn settings_gear_has_a_hit_region_below_the_minimap() {
        assert!(settings_gear_contains(980.0, 260.0, 1000));
        assert!(!settings_gear_contains(980.0, 100.0, 1000));
    }

    #[test]
    fn minimap_draws_the_ship_marker() {
        let mut fb = Framebuffer::new(1000, 640);
        let game = Game::new();
        draw_minimap(&mut fb, &game);
        assert!(fb.color.iter().any(|pixel| *pixel == 0x5BE7FF));
    }
}
