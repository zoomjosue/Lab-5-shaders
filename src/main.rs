use celestial_shaders::{
    framebuffer::Framebuffer,
    renderer::{self, Camera},
    scene::{self, SceneKind},
};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::{
    error::Error,
    time::{Duration, Instant},
};

const PREVIEW_WIDTH: usize = 800;
const PREVIEW_HEIGHT: usize = 600;
struct Options {
    scene: SceneKind,
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = parse_options()?;
    run_window(options.scene)
}

fn parse_options() -> Result<Options, String> {
    let mut scene = SceneKind::VerdantWorld;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scene" => {
                let number = args
                    .next()
                    .ok_or("falta 1, 2, 3 o 4 después de --scene")?
                    .parse::<usize>()
                    .map_err(|_| "la escena debe ser 1, 2, 3 o 4")?;
                scene = SceneKind::from_number(number).ok_or("la escena debe ser 1, 2, 3 o 4")?;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => return Err(format!("argumento desconocido: {arg}")),
        }
    }
    Ok(Options { scene })
}

fn print_help() {
    println!("Uso: cargo run --release -- [--scene 1|2|3|4]");
    println!("1: planeta Viridia, 2: planeta Noctis, 3: gigante Azurón, 4: Sol Helios");
}

fn run_window(initial: SceneKind) -> Result<(), Box<dyn Error>> {
    let mut scene = scene::load(initial, "assets")?;
    let mut framebuffer = Framebuffer::new(PREVIEW_WIDTH, PREVIEW_HEIGHT);
    let mut camera = Camera::default();
    let mut rotation = 0.08_f32;
    let mut auto_rotate = false;
    let start = Instant::now();
    let mut last_animation = Instant::now();
    let mut window = Window::new(
        initial.title(),
        PREVIEW_WIDTH,
        PREVIEW_HEIGHT,
        WindowOptions::default(),
    )?;
    window.set_target_fps(30);
    println!(
        "Renderer paralelo: {} hilos lógicos, 800×600, objetivo 30 FPS",
        renderer::worker_count()
    );
    renderer::render(&mut framebuffer, &scene, camera, 0.0, rotation);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mut changed = false;
        if auto_rotate && last_animation.elapsed() >= Duration::from_millis(33) {
            rotation += 0.008;
            last_animation = Instant::now();
            changed = true;
        }
        if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
            camera.yaw -= 0.025;
            changed = true;
        }
        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
            camera.yaw += 0.025;
            changed = true;
        }
        if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
            camera.pitch = (camera.pitch + 0.02).min(1.2);
            changed = true;
        }
        if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
            camera.pitch = (camera.pitch - 0.02).max(-1.2);
            changed = true;
        }
        if window.is_key_down(Key::Equal) {
            camera.distance_factor = (camera.distance_factor - 0.025).max(0.68);
            changed = true;
        }
        if window.is_key_down(Key::Minus) {
            camera.distance_factor = (camera.distance_factor + 0.025).min(1.8);
            changed = true;
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            camera = Camera::default();
            rotation = 0.08;
            changed = true;
        }
        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            auto_rotate = !auto_rotate;
            last_animation = Instant::now();
            changed = true;
        }

        let selected = if window.is_key_pressed(Key::Key1, KeyRepeat::No) {
            Some(SceneKind::VerdantWorld)
        } else if window.is_key_pressed(Key::Key2, KeyRepeat::No) {
            Some(SceneKind::MechanicalWorld)
        } else if window.is_key_pressed(Key::Key3, KeyRepeat::No) {
            Some(SceneKind::AtomicGiant)
        } else if window.is_key_pressed(Key::Key4, KeyRepeat::No) {
            Some(SceneKind::Sun)
        } else {
            None
        };
        if let Some(kind) = selected {
            scene = scene::load(kind, "assets")?;
            camera = Camera::default();
            rotation = 0.08;
            window.set_title(kind.title());
            changed = true;
        }

        if changed {
            renderer::render(
                &mut framebuffer,
                &scene,
                camera,
                start.elapsed().as_secs_f32(),
                rotation,
            );
        }
        window.update_with_buffer(framebuffer.pixels(), PREVIEW_WIDTH, PREVIEW_HEIGHT)?;
    }
    Ok(())
}
