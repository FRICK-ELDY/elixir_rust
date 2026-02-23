//! Chase AI ベンチマーク: rayon スカラー版 vs SIMD 版

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use game_native::{update_chase_ai, EnemyKind, EnemyWorld};
#[cfg(target_arch = "x86_64")]
use game_native::update_chase_ai_simd;

fn setup_enemies(n: usize) -> EnemyWorld {
    let mut enemies = EnemyWorld::new();
    let positions: Vec<(f32, f32)> = (0..n)
        .map(|i| {
            let x = (i as f32 * 1.7) % 1280.0;
            let y = (i as f32 * 2.3) % 720.0;
            (x, y)
        })
        .collect();
    enemies.spawn(&positions, EnemyKind::Slime);
    enemies
}

fn bench_chase_ai(c: &mut Criterion) {
    let n = 10_000;
    let player_x = 640.0;
    let player_y = 360.0;
    let dt = 0.016;

    c.bench_function("chase_ai_scalar_rayon", |b| {
        b.iter_batched(
            || setup_enemies(n),
            |mut enemies| {
                update_chase_ai(&mut enemies, player_x, player_y, dt);
                enemies
            },
            BatchSize::PerIteration,
        )
    });

    #[cfg(target_arch = "x86_64")]
    c.bench_function("chase_ai_simd", |b| {
        b.iter_batched(
            || setup_enemies(n),
            |mut enemies| {
                update_chase_ai_simd(&mut enemies, player_x, player_y, dt);
                enemies
            },
            BatchSize::PerIteration,
        )
    });
}

criterion_group!(benches, bench_chase_ai);
criterion_main!(benches);
