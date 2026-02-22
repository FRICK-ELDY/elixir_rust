mod constants;

use constants::{PLAYER_SIZE, PLAYER_SPEED, SCREEN_HEIGHT, SCREEN_WIDTH};
use rustler::{Atom, NifResult, ResourceArc};
use std::sync::Mutex;

rustler::atoms! {
    ok
}

pub struct PlayerState {
    pub x: f32,
    pub y: f32,
    pub input_dx: f32,
    pub input_dy: f32,
}

pub struct GameWorldInner {
    pub frame_id: u32,
    pub player: PlayerState,
}

pub struct GameWorld(pub Mutex<GameWorldInner>);

#[rustler::nif]
fn add(a: i64, b: i64) -> NifResult<i64> {
    Ok(a + b)
}

#[rustler::nif]
fn create_world() -> ResourceArc<GameWorld> {
    ResourceArc::new(GameWorld(Mutex::new(GameWorldInner {
        frame_id: 0,
        player: PlayerState {
            x: SCREEN_WIDTH  / 2.0 - PLAYER_SIZE / 2.0,
            y: SCREEN_HEIGHT / 2.0 - PLAYER_SIZE / 2.0,
            input_dx: 0.0,
            input_dy: 0.0,
        },
    })))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn physics_step(world: ResourceArc<GameWorld>, delta_ms: f64) -> u32 {
    let mut w = world.0.lock().unwrap();
    w.frame_id += 1;

    let dt = delta_ms as f32 / 1000.0;
    let dx = w.player.input_dx;
    let dy = w.player.input_dy;

    // 斜め移動を正規化して速度を一定に保つ
    let len = (dx * dx + dy * dy).sqrt();
    if len > 0.001 {
        w.player.x += (dx / len) * PLAYER_SPEED * dt;
        w.player.y += (dy / len) * PLAYER_SPEED * dt;
    }

    w.player.x = w.player.x.clamp(0.0, SCREEN_WIDTH  - PLAYER_SIZE);
    w.player.y = w.player.y.clamp(0.0, SCREEN_HEIGHT - PLAYER_SIZE);

    w.frame_id
}

#[rustler::nif]
fn set_player_input(world: ResourceArc<GameWorld>, dx: f64, dy: f64) -> Atom {
    let mut w = world.0.lock().unwrap();
    w.player.input_dx = dx as f32;
    w.player.input_dy = dy as f32;
    ok()
}

#[rustler::nif]
fn get_player_pos(world: ResourceArc<GameWorld>) -> (f64, f64) {
    let w = world.0.lock().unwrap();
    (w.player.x as f64, w.player.y as f64)
}

#[allow(non_local_definitions)]
fn load(env: rustler::Env, _: rustler::Term) -> bool {
    let _ = rustler::resource!(GameWorld, env);
    true
}

rustler::init!("Elixir.Game.NifBridge", load = load);
