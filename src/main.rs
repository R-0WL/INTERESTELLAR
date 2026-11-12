//! Interstellar Journey - Rust software-rendering edition.
//!
//! The app starts in a keyboard-operated mode menu. Exploration uses the
//! custom CPU rasterizer; the menu, HUD, and minimap are all framebuffer UI.

mod framebuffer;
mod game;
mod math;
mod mesh;
mod renderer;
mod ui;

use crate::framebuffer::Framebuffer;
use crate::game::{Game, InputState};
use crate::mesh::load_obj;
use crate::renderer::{ImportedModels, Renderer};
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const WIDTH: usize = 1000;
const HEIGHT: usize = 640;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppScreen {
    MainMenu,
    Exploration,
    StoryPreview,
    LevelsPreview,
    Settings,
}

#[derive(Clone, Copy, Debug)]
struct SettingsState {
    music_enabled: bool,
    dialogue_visible: bool,
    effects_enabled: bool,
    shader_layer: u8,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            music_enabled: true,
            dialogue_visible: true,
            effects_enabled: true,
            shader_layer: 1,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.iter().any(|argument| argument == "--help" || argument == "-h") {
        print_help();
        return Ok(());
    }

    let model_path = parse_model_argument(&arguments).unwrap_or_else(|| PathBuf::from("assets/models/esfera.obj"));
    let startup_place = parse_named_option(&arguments, "--place", false).map_err(invalid_input)?;
    let photo_option = parse_named_option(&arguments, "--foto", true).map_err(invalid_input)?;
    let shader_layer = parse_shader_layer(&arguments).map_err(invalid_input)?;
    let photo_targets = photo_option.map(|value| expand_photo_targets(&value)).unwrap_or_default();

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

    let object_models = ImportedModels {
        solar_ship: load_optional_obj("assets/models/objs/endurance_ranger_ranger.obj"),
        wormhole_ship: load_optional_obj("assets/models/objs/ranger.obj"),
        gargantua: load_optional_obj("assets/models/objs/blackhole.obj"),
        accretion_disc: load_optional_obj("assets/blackhole_dorado/disco_capas.obj"),
        endurance: load_optional_obj("assets/models/objs/endurance_ranger.obj"),
        asteroid: load_optional_obj("assets/models/objs/asteroid.obj"),
    };

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let renderer = Renderer::new(WIDTH, HEIGHT, sphere, object_models);
    let mut game = Game::new();

    if let Some(place) = startup_place.as_deref() {
        game.place_named(place).map_err(invalid_input)?;
        println!("Modo exploracion iniciado en: {place}");
    }

    // Batch photography is completely headless: it renders the requested
    // framebuffer directly and does not open the interactive minifb window.
    if !photo_targets.is_empty() {
        fs::create_dir_all("captures")?;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        for target in &photo_targets {
            let mut photo_game = Game::new();
            photo_game.place_named(target).map_err(invalid_input)?;
            renderer.render_with_layer(&mut framebuffer, &photo_game, shader_layer);
            let safe_target = sanitize_filename(target);
            let filename = format!("captures/interstellar_{safe_target}_layer_{shader_layer}_{timestamp}.bmp");
            framebuffer.save_bmp(&filename)?;
            println!("Fotografia guardada: {filename}");
        }
        return Ok(());
    }

    let mut window = Window::new(
        "INTERSTELLAR JOURNEY | Rust Software Renderer",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )?;
    window.set_target_fps(60);

    let mut screen = if startup_place.is_some() { AppScreen::Exploration } else { AppScreen::MainMenu };
    let mut settings_return_screen = AppScreen::MainMenu;
    let mut selected_mode = 0usize;
    let mut selected_setting = 0usize;
    let mut settings = SettingsState::default();
    settings.shader_layer = shader_layer;
    let mut show_minimap = true;

    // Edge-triggered keys keep menu selection and toggles from repeating every frame.
    let mut previous_up = false;
    let mut previous_down = false;
    let mut previous_left = false;
    let mut previous_right = false;
    let mut previous_enter = false;
    let mut previous_backspace = false;
    let mut previous_m = false;
    let mut previous_g = false;
    let mut previous_mouse_left = false;
    let mut previous_e = false;
    let mut previous_space = false;
    let mut previous_r = false;
    let mut previous_p = false;
    let mut previous_frame = Instant::now();

    println!("\nINTERSTELLAR JOURNEY - RUST");
    println!("Menu: Up/Down + Enter | Settings: Up/Down + Left/Right | Exploration: W/S thrust, A/D steer, arrows pitch");
    println!("W + Shift: BOOST (1000) | W + Shift+B: SUPERBOOST (2500) | E: orbit assist | Space: wormhole | R: restart");
    println!("G / gear: settings | M: show/hide minimap | Backspace: main menu | P: save BMP | Esc: exit\n");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = (now - previous_frame).as_secs_f32().min(0.25);
        previous_frame = now;

        let key_up = window.is_key_down(Key::Up);
        let key_down = window.is_key_down(Key::Down);
        let key_left = window.is_key_down(Key::Left);
        let key_right = window.is_key_down(Key::Right);
        let key_enter = window.is_key_down(Key::Enter);
        let key_g = window.is_key_down(Key::G);
        let mouse_down = window.get_mouse_down(MouseButton::Left);
        let mouse_clicked = mouse_down && !previous_mouse_left;
        let mouse_position = window.get_mouse_pos(MouseMode::Discard);
        let key_backspace = window.is_key_down(Key::Backspace);
        let key_m = window.is_key_down(Key::M);
        let key_e = window.is_key_down(Key::E);
        let key_space = window.is_key_down(Key::Space);
        let key_r = window.is_key_down(Key::R);
        let key_p = window.is_key_down(Key::P);

        let up_pressed = key_up && !previous_up;
        let down_pressed = key_down && !previous_down;
        let left_pressed = key_left && !previous_left;
        let right_pressed = key_right && !previous_right;
        let enter_pressed = key_enter && !previous_enter;
        let backspace_pressed = key_backspace && !previous_backspace;
        let m_pressed = key_m && !previous_m;
        let g_pressed = key_g && !previous_g;
        let gear_clicked = mouse_clicked && mouse_position
            .map(|(x, y)| ui::settings_gear_contains(x, y, WIDTH))
            .unwrap_or(false);

        let mut screenshot_requested = false;

        match screen {
            AppScreen::MainMenu => {
                if up_pressed {
                    selected_mode = (selected_mode + 3) % 4;
                }
                if down_pressed {
                    selected_mode = (selected_mode + 1) % 4;
                }
                if enter_pressed {
                    screen = match selected_mode {
                        0 => AppScreen::Exploration,
                        1 => AppScreen::StoryPreview,
                        2 => AppScreen::LevelsPreview,
                        _ => {
                            settings_return_screen = AppScreen::MainMenu;
                            selected_setting = 0;
                            AppScreen::Settings
                        }
                    };
                }
            }
            AppScreen::StoryPreview | AppScreen::LevelsPreview => {
                if enter_pressed || backspace_pressed {
                    screen = AppScreen::MainMenu;
                }
            }
            AppScreen::Settings => {
                if backspace_pressed {
                    screen = settings_return_screen;
                } else {
                    if up_pressed {
                        selected_setting = (selected_setting + 3) % 4;
                    }
                    if down_pressed {
                        selected_setting = (selected_setting + 1) % 4;
                    }
                    match selected_setting {
                        0 if left_pressed || right_pressed || enter_pressed => settings.music_enabled = !settings.music_enabled,
                        1 if left_pressed || right_pressed || enter_pressed => settings.dialogue_visible = !settings.dialogue_visible,
                        2 if left_pressed || right_pressed || enter_pressed => settings.effects_enabled = !settings.effects_enabled,
                        3 if left_pressed => settings.shader_layer = if settings.shader_layer <= 1 { 6 } else { settings.shader_layer - 1 },
                        3 if right_pressed || enter_pressed => settings.shader_layer = settings.shader_layer % 6 + 1,
                        _ => {}
                    }
                }
            }
            AppScreen::Exploration => {
                if backspace_pressed {
                    screen = AppScreen::MainMenu;
                } else if g_pressed || gear_clicked {
                    settings_return_screen = AppScreen::Exploration;
                    selected_setting = 0;
                    screen = AppScreen::Settings;
                } else {
                    if m_pressed {
                        show_minimap = !show_minimap;
                    }

                    let input = InputState {
                        forward: window.is_key_down(Key::W),
                        back: window.is_key_down(Key::S),
                        left: window.is_key_down(Key::A) || window.is_key_down(Key::Left),
                        right: window.is_key_down(Key::D) || window.is_key_down(Key::Right),
                        pitch_up: window.is_key_down(Key::Up),
                        pitch_down: window.is_key_down(Key::Down),
                        boost: window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift),
                        superboost: window.is_key_down(Key::B),
                        orbit_pressed: key_e && !previous_e,
                        wormhole_pressed: key_space && !previous_space,
                        restart_pressed: key_r && !previous_r,
                        screenshot_pressed: key_p && !previous_p,
                    };

                    game.update(dt, input);
                    screenshot_requested = input.screenshot_pressed;
                }
            }
        }

        // Render whichever screen is active after handling this frame's input.
        match screen {
            AppScreen::MainMenu => ui::draw_main_menu(&mut framebuffer, selected_mode),
            AppScreen::StoryPreview => ui::draw_mode_preview(
                &mut framebuffer,
                "Modo historia",
                "Una campana narrativa con escenas, dialogos y objetivos de mision.",
            ),
            AppScreen::LevelsPreview => ui::draw_mode_preview(
                &mut framebuffer,
                "Modo niveles",
                "Desafios independientes con rutas, tiempos y condiciones de llegada.",
            ),
            AppScreen::Settings => ui::draw_settings_menu(
                &mut framebuffer,
                selected_setting,
                settings.music_enabled,
                settings.dialogue_visible,
                settings.effects_enabled,
                settings.shader_layer,
            ),
            AppScreen::Exploration => {
                renderer.render_with_layer(&mut framebuffer, &game, settings.shader_layer);
                ui::draw_hud(&mut framebuffer, &game, &game.hud(), game.capture_notice, show_minimap);

                if screenshot_requested {
                    fs::create_dir_all("captures")?;
                    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
                    let filename = format!("captures/interstellar_{timestamp}.bmp");
                    framebuffer.save_bmp(&filename)?;
                    println!("Captura guardada en {filename}");
                }
            }
        }

        window.update_with_buffer(&framebuffer.color, WIDTH, HEIGHT)?;

        previous_up = key_up;
        previous_down = key_down;
        previous_left = key_left;
        previous_right = key_right;
        previous_enter = key_enter;
        previous_backspace = key_backspace;
        previous_m = key_m;
        previous_g = key_g;
        previous_mouse_left = mouse_down;
        previous_e = key_e;
        previous_space = key_space;
        previous_r = key_r;
        previous_p = key_p;
    }

    Ok(())
}

fn invalid_input(message: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}

fn parse_model_argument(arguments: &[String]) -> Option<PathBuf> {
    arguments.windows(2)
        .find(|pair| pair[0] == "--obj")
        .map(|pair| PathBuf::from(&pair[1]))
}

fn parse_named_option(arguments: &[String], flag: &str, default_all: bool) -> Result<Option<String>, String> {
    let bracket_prefix = format!("{flag}[");
    let equals_prefix = format!("{flag}=");
    for (index, argument) in arguments.iter().enumerate() {
        if argument == flag {
            if let Some(value) = arguments.get(index + 1).filter(|value| !value.starts_with("--")) {
                return Ok(Some(value.clone()));
            }
            return if default_all {
                Ok(Some("all".to_string()))
            } else {
                Err(format!("Falta el nombre del lugar después de {flag}. Usa --help para ver ejemplos."))
            };
        }
        if let Some(value) = argument.strip_prefix(equals_prefix.as_str()) {
            return Ok(Some(value.trim().to_string()));
        }
        if let Some(value) = argument.strip_prefix(bracket_prefix.as_str()).and_then(|value| value.strip_suffix(']')) {
            return Ok(Some(value.trim().to_string()));
        }
    }
    Ok(None)
}

fn parse_shader_layer(arguments: &[String]) -> Result<u8, String> {
    let mut value = None;
    for (index, argument) in arguments.iter().enumerate() {
        if argument == "--shader-layer" || argument == "--layer" {
            value = Some(arguments.get(index + 1)
                .filter(|next| !next.starts_with("--"))
                .cloned()
                .ok_or_else(|| "Falta el número de capa después de --shader-layer".to_string())?);
        } else if let Some(layer) = argument.strip_prefix("--shader-layer=").or_else(|| argument.strip_prefix("--layer=")) {
            value = Some(layer.to_string());
        }
    }
    let layer = value.map(|value| value.parse::<u8>().map_err(|_| "La capa shader debe ser un número entre 1 y 6".to_string())).transpose()?.unwrap_or(1);
    if (1..=6).contains(&layer) {
        Ok(layer)
    } else {
        Err("La capa shader debe estar entre 1 y 6".to_string())
    }
}

fn expand_photo_targets(value: &str) -> Vec<String> {
    const ALL_TARGETS: &[&str] = &[
        "mercury", "venus", "earth", "moon", "mars", "jupiter", "saturn", "uranus", "neptune",
        "miller", "mann", "edmund", "sun", "wormhole", "gargantua", "endurance",
        "galaxy2-top", "galaxy2-side", "galaxy1-top", "galaxy1-side",
    ];
    if value.trim().eq_ignore_ascii_case("all") || value.trim().is_empty() {
        return ALL_TARGETS.iter().map(|target| (*target).to_string()).collect();
    }
    let requested: Vec<String> = value.split(|character| character == ',' || character == ';')
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if requested.iter().any(|target| target.eq_ignore_ascii_case("all")) {
        ALL_TARGETS.iter().map(|target| (*target).to_string()).collect()
    } else {
        requested
    }
}

fn sanitize_filename(value: &str) -> String {
    value.chars().map(|character| {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            character.to_ascii_lowercase()
        } else {
            '-'
        }
    }).collect()
}

fn print_help() {
    println!("INTERSTELLAR JOURNEY - CLI\n");
    println!("  cargo run --release -- --place earth");
    println!("  cargo run --release -- --place[galaxy2-top]");
    println!("  cargo run --release -- --place=galaxy2-top");
    println!("  cargo run --release -- --foto earth");
    println!("  cargo run --release -- --foto=earth,moon,sun");
    println!("  cargo run --release -- --foto all --shader-layer 1");
    println!("  cargo run --release -- --foto[all]");
    println!("\n--place targets: mercury, venus, earth, moon, mars, jupiter, saturn, uranus, neptune,");
    println!("miller, mann, edmund, sun, wormhole, gargantua, endurance,");
    println!("galaxy1-top, galaxy1-side, galaxy2-top, galaxy2-side.");
    println!("--foto accepts the same targets, comma/semicolon lists or 'all'.");
    println!("--shader-layer / --layer accepts values 1..6 (default: 1). --obj PATH loads a sphere OBJ.");
}

fn load_optional_obj(path: &str) -> Option<crate::mesh::Mesh> {
    match load_obj(path) {
        Ok(mesh) => {
            println!(
                "Modelo OBJ cargado: {} ({} vertices, {} triangulos)",
                path,
                mesh.vertices.len(),
                mesh.triangles.len()
            );
            Some(mesh)
        }
        Err(error) => {
            eprintln!("Aviso: {error}. Se usara la geometria de respaldo.");
            None
        }
    }
}


#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn named_options_accept_separate_equals_and_bracket_forms() {
        assert_eq!(parse_named_option(&["--place".into(), "earth".into()], "--place", false).unwrap(), Some("earth".into()));
        assert_eq!(parse_named_option(&["--place=moon".into()], "--place", false).unwrap(), Some("moon".into()));
        assert_eq!(parse_named_option(&["--place[galaxy2-top]".into()], "--place", false).unwrap(), Some("galaxy2-top".into()));
    }

    #[test]
    fn foto_without_target_defaults_to_all_targets() {
        assert_eq!(parse_named_option(&["--foto".into()], "--foto", true).unwrap(), Some("all".into()));
        assert_eq!(expand_photo_targets("all").len(), 20);
    }

    #[test]
    fn photo_target_lists_accept_commas_and_semicolons() {
        assert_eq!(expand_photo_targets("earth, moon;galaxy1-side"), vec![String::from("earth"), String::from("moon"), String::from("galaxy1-side")]);
    }

    #[test]
    fn shader_layer_accepts_all_reserved_slots_and_rejects_invalid_values() {
        for layer in 1..=6 {
            assert_eq!(parse_shader_layer(&["--shader-layer".into(), layer.to_string()]).unwrap(), layer);
        }
        assert!(parse_shader_layer(&["--shader-layer".into(), "7".into()]).is_err());
        assert!(parse_shader_layer(&["--shader-layer".into()]).is_err());
    }
}
