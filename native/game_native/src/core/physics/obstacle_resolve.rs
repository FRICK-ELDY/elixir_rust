//! Path: native/game_native/src/core/physics/obstacle_resolve.rs
//! Summary: プレイヤーと障害物の衝突解決・押し出し処理
//!
//! 1.5.2: lib / main 共通で使用。

use super::spatial_hash::CollisionWorld;
use crate::core::constants::PLAYER_RADIUS;

/// プレイヤーが障害物と重なっている場合に押し出す（複数障害物対応）
pub fn resolve_obstacles_player(
    collision: &CollisionWorld,
    player_x: &mut f32,
    player_y: &mut f32,
    buf: &mut Vec<usize>,
) {
    for _ in 0..5 {
        let cx = *player_x + PLAYER_RADIUS;
        let cy = *player_y + PLAYER_RADIUS;
        collision.query_static_nearby_into(cx, cy, PLAYER_RADIUS, buf);
        let mut pushed = false;
        for &idx in buf.iter() {
            if let Some(o) = collision.obstacles.get(idx) {
                let dx = cx - o.x;
                let dy = cy - o.y;
                let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                let overlap = (PLAYER_RADIUS + o.radius) - dist;
                if overlap > 0.0 {
                    *player_x += (dx / dist) * overlap;
                    *player_y += (dy / dist) * overlap;
                    pushed = true;
                    break;
                }
            }
        }
        if !pushed {
            break;
        }
    }
}
