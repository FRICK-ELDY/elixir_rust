//! Path: native/game_core/src/physics/separation.rs
//! Summary: 敵同士の重なり解消（Separation）トレイトと適用ロジック

use super::spatial_hash::SpatialHash;

pub trait EnemySeparation {
    fn enemy_count(&self) -> usize;
    fn is_alive(&self, i: usize) -> bool;
    fn pos_x(&self, i: usize) -> f32;
    fn pos_y(&self, i: usize) -> f32;
    fn add_pos_x(&mut self, i: usize, v: f32);
    fn add_pos_y(&mut self, i: usize, v: f32);
    fn sep_buf_x(&mut self) -> &mut Vec<f32>;
    fn sep_buf_y(&mut self) -> &mut Vec<f32>;
    fn neighbor_buf(&mut self) -> &mut Vec<usize>;
}

pub fn apply_separation<W: EnemySeparation>(
    world: &mut W,
    separation_radius: f32,
    separation_force: f32,
    dt: f32,
) {
    let len = world.enemy_count();
    if len < 2 {
        return;
    }

    world.sep_buf_x().iter_mut().for_each(|v| *v = 0.0);
    world.sep_buf_y().iter_mut().for_each(|v| *v = 0.0);

    let mut hash = SpatialHash::new(separation_radius);
    for i in 0..len {
        if world.is_alive(i) {
            hash.insert(i, world.pos_x(i), world.pos_y(i));
        }
    }

    for i in 0..len {
        if !world.is_alive(i) {
            continue;
        }
        let ix = world.pos_x(i);
        let iy = world.pos_y(i);

        hash.query_nearby_into(ix, iy, separation_radius, world.neighbor_buf());
        let nb_len = world.neighbor_buf().len();
        for ni in 0..nb_len {
            let j = world.neighbor_buf()[ni];
            if j <= i || !world.is_alive(j) {
                continue;
            }
            let jx = world.pos_x(j);
            let jy = world.pos_y(j);

            let dx = ix - jx;
            let dy = iy - jy;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < separation_radius * separation_radius && dist_sq > 1e-6 {
                let dist = dist_sq.sqrt();
                let overlap = separation_radius - dist;
                let force = overlap * separation_force * dt;
                let nx = (dx / dist) * force;
                let ny = (dy / dist) * force;
                world.sep_buf_x()[i] += nx;
                world.sep_buf_y()[i] += ny;
                world.sep_buf_x()[j] -= nx;
                world.sep_buf_y()[j] -= ny;
            }
        }
    }

    for i in 0..len {
        if world.is_alive(i) {
            let sx = world.sep_buf_x()[i];
            let sy = world.sep_buf_y()[i];
            world.add_pos_x(i, sx);
            world.add_pos_y(i, sy);
        }
    }
}
