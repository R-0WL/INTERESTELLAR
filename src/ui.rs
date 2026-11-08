//! Minimal HUD drawn directly into the framebuffer (no browser/GUI overlay).

use crate::framebuffer::Framebuffer;
use crate::game::HudState;
use font8x8::{UnicodeFonts, BASIC_FONTS};

pub fn draw_hud(framebuffer: &mut Framebuffer, state: &HudState, capture_notice: f32) {
    let width = framebuffer.width as i32;
    let height = framebuffer.height as i32;

    // Mission information panel.
    let panel_width = 370.min(width - 24);
    framebuffer.fill_rect(16, 14, panel_width, 112, 0x07111F);
    framebuffer.draw_rect(16, 14, panel_width, 112, 0x42647D);
    draw_text(framebuffer, 28, 24, "MISSION TRAJECTORY", 0x79CFFF, 1);
    draw_text(framebuffer, 28, 44, &truncate(&state.title, 20), 0xFFFFFF, 2);
    draw_text(framebuffer, 28, 78, &truncate(&state.body, 42), 0xC4D4E0, 1);
    draw_text(framebuffer, 28, 95, &truncate(&state.prompt, 42), 0xFFD98E, 1);

    // Telemetry panel.
    let telemetry_x = (width - 218).max(16);
    framebuffer.fill_rect(telemetry_x, 14, 202, 112, 0x07111F);
    framebuffer.draw_rect(telemetry_x, 14, 202, 112, 0x42647D);
    draw_text(framebuffer, telemetry_x + 12, 24, "FLIGHT DATA", 0x79CFFF, 1);
    draw_text(framebuffer, telemetry_x + 12, 44, &format!("SPEED  {:>5.0} U/S", state.speed), 0xFFFFFF, 1);
    draw_text(framebuffer, telemetry_x + 12, 62, &format!("RANGE  {:>7.0}", state.distance), 0xFFFFFF, 1);
    draw_text(framebuffer, telemetry_x + 12, 80, &format!("PHASE  {}", state.phase), 0xFFFFFF, 1);
    let warning_color = if state.warning.is_empty() { 0x80DCA9 } else { 0xFF6C64 };
    let warning: &str = if state.warning.is_empty() { "STATUS  NOMINAL" } else { state.warning.as_str() };
    draw_text(framebuffer, telemetry_x + 12, 99, warning, warning_color, 1);

    // Center reticle.
    let center_x = width / 2;
    let center_y = height / 2;
    let reticle = 0x9DDFFF;
    framebuffer.draw_rect(center_x - 9, center_y - 9, 18, 18, reticle);
    framebuffer.draw_line(center_x - 15, center_y, center_x - 11, center_y, reticle);
    framebuffer.draw_line(center_x + 11, center_y, center_x + 15, center_y, reticle);
    framebuffer.draw_line(center_x, center_y - 15, center_x, center_y - 11, reticle);
    framebuffer.draw_line(center_x, center_y + 11, center_x, center_y + 15, reticle);

    // Controls strip and contextual prompt.
    let controls_y = height - 28;
    framebuffer.fill_rect(12, controls_y - 7, width - 24, 24, 0x07111F);
    draw_text(
        framebuffer,
        22,
        controls_y,
        "W/S THRUST   A/D STEER   ARROWS PITCH   SHIFT BOOST   E LAND   SPACE WORMHOLE   R TAKEOFF",
        0xA8BBCB,
        1,
    );

    if capture_notice > 0.0 {
        let message = "SCREENSHOT SAVED: captures/";
        let x = (width - (message.len() as i32 * 8)) / 2;
        framebuffer.fill_rect(x - 8, 142, message.len() as i32 * 8 + 16, 24, 0x0B1C2B);
        framebuffer.draw_rect(x - 8, 142, message.len() as i32 * 8 + 16, 24, 0x64C9FF);
        draw_text(framebuffer, x, 150, message, 0xDDF5FF, 1);
    }

    if height > 220 {
        draw_text(framebuffer, 22, height - 48, &format!("TARGET  {}", truncate(&state.target, 38)), 0x7EABC9, 1);
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
}
