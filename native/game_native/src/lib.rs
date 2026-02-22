use rustler::{NifResult, ResourceArc};
use std::sync::Mutex;

pub struct GameWorldInner {
    pub frame_id: u32,
}

pub struct GameWorld(pub Mutex<GameWorldInner>);

#[rustler::nif]
fn add(a: i64, b: i64) -> NifResult<i64> {
    Ok(a + b)
}

#[rustler::nif]
fn create_world() -> ResourceArc<GameWorld> {
    ResourceArc::new(GameWorld(Mutex::new(GameWorldInner { frame_id: 0 })))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn physics_step(world: ResourceArc<GameWorld>, _delta_ms: f64) -> u32 {
    let mut w = world.0.lock().unwrap();
    w.frame_id += 1;
    w.frame_id
}

#[allow(non_local_definitions)]
fn load(env: rustler::Env, _: rustler::Term) -> bool {
    let _ = rustler::resource!(GameWorld, env);
    true
}

rustler::init!("Elixir.Game.NifBridge", load = load);
