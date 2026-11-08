//! Interstellar Journey - Rust software-rendering edition.
//!
//! Controls are polled by `minifb`; all 3D geometry is drawn by our CPU
//! rasterizer into an owned framebuffer and then presented in one window.

mod framebuffer;
mod game;
mod math;
mod mesh;
mod renderer;
mod ui;

use crate::framebuffer::Framebuffer;
use crate::game::{Game, InputState};
use crate::mesh::load_obj;
use crate::renderer::Renderer;
use minifb::{Key, Window, WindowOptions};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const WIDTH: usize = 1000;
const HEIGHT: usize = 640;

fn main() -> Result<(), Box<dyn Error>> {
    let model_path = parse_model_argument().unwrap_or_else(|| PathBuf::from("assets/models/esfera.obj"));
    let sphere = match load_obj(&model_path) {
        Ok(mesh) => {
            println!(
                "Modelo OBJ cargado: {} ({} vertices, {} triangulos)",
                model_path.display(),
                mesh.vertices.len(),
                mesh.triangles.len()
            );
            Some(mesh)
        }
        Err(error) => {
            eprintln!("Aviso: {error}. Se usara una esfera generada por codigo.");
            None
        }
    };

    let mut window = Window::new(
        "INTERSTELLAR JOURNEY | Rust Software Renderer",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )?;
    window.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let renderer = Renderer::new(WIDTH, HEIGHT, sphere);
    let mut game = Game::new();
    let mut previous_e = false;
    let mut previous_space = false;
    let mut previous_r = false;
    let mut previous_p = false;
    let mut previous_frame = Instant::now();

    println!("\nINTERSTELLAR JOURNEY - RUST");
    println!("W/S: thrust/brake | A/D: steer | Arrows: pitch | Shift: boost");
    println!("E: land | Space: cross the wormhole | R: take off/restart | P: save BMP | Esc: exit\n");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = (now - previous_frame).as_secs_f32().min(0.05);
        previous_frame = now;

        let key_e = window.is_key_down(Key::E);
        let key_space = window.is_key_down(Key::Space);
        let key_r = window.is_key_down(Key::R);
        let key_p = window.is_key_down(Key::P);

        let input = InputState {
            forward: window.is_key_down(Key::W),
            back: window.is_key_down(Key::S),
            left: window.is_key_down(Key::A) || window.is_key_down(Key::Left),
            right: window.is_key_down(Key::D) || window.is_key_down(Key::Right),
            pitch_up: window.is_key_down(Key::Up),
            pitch_down: window.is_key_down(Key::Down),
            boost: window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift),
            interact_pressed: key_e && !previous_e,
            wormhole_pressed: key_space && !previous_space,
            takeoff_pressed: key_r && !previous_r,
            screenshot_pressed: key_p && !previous_p,
        };
        previous_e = key_e;
        previous_space = key_space;
        previous_r = key_r;
        previous_p = key_p;

        game.update(dt, input);
        renderer.render(&mut framebuffer, &game);
        ui::draw_hud(&mut framebuffer, &game.hud(), game.capture_notice);

        if input.screenshot_pressed {
            fs::create_dir_all("captures")?;
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let filename = format!("captures/interstellar_{timestamp}.bmp");
            framebuffer.save_bmp(&filename)?;
            println!("Captura guardada en {filename}");
        }

        window.update_with_buffer(&framebuffer.color, WIDTH, HEIGHT)?;
    }

    Ok(())
}

fn parse_model_argument() -> Option<PathBuf> {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--obj" {
            return arguments.next().map(PathBuf::from);
        }
    }
    None
}
