//! Path: native/game_native/benches/load_world_bench.rs
//! Summary: 敵/弾/パーティクル増量時の physics_step ベンチマーク

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use game_core::constants::{CELL_SIZE, PARTICLE_RNG_SEED, PLAYER_SIZE, SCREEN_HEIGHT, SCREEN_WIDTH};
use game_core::item::ItemWorld;
use game_core::physics::rng::SimpleRng;
use game_core::physics::spatial_hash::CollisionWorld;
use game_core::weapon::WeaponSlot;
use game_native::{
    run_physics_step_for_bench, BulletWorld, EnemyWorld, GameWorldInner, ParticleWorld, PlayerState,
};
use std::sync::Mutex;

#[derive(Clone, Copy)]
struct Scenario {
    name: &'static str,
    enemies: usize,
    bullets: usize,
    particles: usize,
}

fn build_world(s: Scenario) -> GameWorldInner {
    let mut enemies = EnemyWorld::new();
    let enemy_positions: Vec<(f32, f32)> = (0..s.enemies)
        .map(|i| {
            let x = ((i * 13) % 2400) as f32 + 100.0;
            let y = ((i * 17) % 1400) as f32 + 100.0;
            (x, y)
        })
        .collect();
    enemies.spawn(&enemy_positions, 0);

    let mut bullets = BulletWorld::new();
    for i in 0..s.bullets {
        let angle = (i as f32 * 0.017).sin();
        let vx = 240.0 * angle;
        let vy = 240.0 * (1.0 - angle.abs());
        bullets.spawn(
            SCREEN_WIDTH * 0.5,
            SCREEN_HEIGHT * 0.5,
            vx,
            vy,
            10,
            0.8,
            0,
        );
    }

    let mut particles = ParticleWorld::new(PARTICLE_RNG_SEED);
    for i in 0..s.particles {
        let t = i as f32;
        particles.spawn_one(
            400.0 + (t * 0.11).sin() * 300.0,
            300.0 + (t * 0.07).cos() * 200.0,
            (t * 0.03).sin() * 80.0,
            (t * 0.05).cos() * 80.0,
            0.5,
            [1.0, 0.8, 0.2, 1.0],
            4.0,
        );
    }

    GameWorldInner {
        frame_id: 0,
        player: PlayerState {
            x: SCREEN_WIDTH / 2.0 - PLAYER_SIZE / 2.0,
            y: SCREEN_HEIGHT / 2.0 - PLAYER_SIZE / 2.0,
            input_dx: 1.0,
            input_dy: 0.0,
            hp: 100.0,
            invincible_timer: 0.0,
        },
        enemies,
        bullets,
        particles,
        items: ItemWorld::new(),
        magnet_timer: 0.0,
        rng: SimpleRng::new(42),
        collision: CollisionWorld::new(CELL_SIZE),
        obstacle_query_buf: Vec::new(),
        last_frame_time_ms: 0.0,
        score: 0,
        elapsed_seconds: 0.0,
        player_max_hp: 100.0,
        exp: 0,
        level: 1,
        level_up_pending: false,
        weapon_slots: vec![WeaponSlot::new(0)],
        boss: None,
        frame_events: Vec::new(),
        pending_ui_action: Mutex::new(None),
        weapon_choices: Vec::new(),
        score_popups: Vec::new(),
        kill_count: 0,
    }
}

fn bench_world_load(c: &mut Criterion) {
    let scenarios = [
        Scenario { name: "world_load_medium", enemies: 3_000, bullets: 800, particles: 1_000 },
        Scenario { name: "world_load_high", enemies: 6_000, bullets: 1_600, particles: 2_000 },
        Scenario { name: "world_load_extreme", enemies: 10_000, bullets: 2_000, particles: 2_500 },
    ];

    for scenario in scenarios {
        c.bench_function(scenario.name, |b| {
            b.iter_batched(
                || build_world(scenario),
                |mut world| {
                    run_physics_step_for_bench(&mut world, 1000.0 / 60.0);
                    world
                },
                BatchSize::PerIteration,
            )
        });
    }
}

criterion_group!(benches, bench_world_load);
criterion_main!(benches);
