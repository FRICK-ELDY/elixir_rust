/// Standalone rendering binary.
/// Runs the full game loop in pure Rust without Elixir/NIF.
/// Used for renderer development and visual testing.
mod constants;
mod renderer;
mod physics;
mod weapon;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use constants::{
    BULLET_LIFETIME, BULLET_RADIUS, BULLET_SPEED,
    CELL_SIZE, ENEMY_SEPARATION_FORCE,
    ENEMY_SEPARATION_RADIUS, INVINCIBLE_DURATION,
    MAX_ENEMIES, PLAYER_RADIUS, PLAYER_SIZE, PLAYER_SPEED,
    SCREEN_HEIGHT, SCREEN_WIDTH, WAVES,
};
use renderer::{HudData, Renderer};
use weapon::{WeaponKind, WeaponSlot, MAX_WEAPON_LEVEL, MAX_WEAPON_SLOTS};

// ─── 敵タイプ（main.rs ローカル定義） ─────────────────────────
#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum EnemyKind {
    #[default]
    Slime,
    Bat,
    Golem,
}

impl EnemyKind {
    fn max_hp(self) -> f32 {
        match self { Self::Slime => 30.0, Self::Bat => 15.0, Self::Golem => 150.0 }
    }
    fn speed(self) -> f32 {
        match self { Self::Slime => 80.0, Self::Bat => 160.0, Self::Golem => 40.0 }
    }
    fn radius(self) -> f32 {
        match self { Self::Slime => 20.0, Self::Bat => 12.0, Self::Golem => 32.0 }
    }
    fn exp_reward(self) -> u32 {
        match self { Self::Slime => 5, Self::Bat => 3, Self::Golem => 20 }
    }
    fn damage_per_sec(self) -> f32 {
        match self { Self::Slime => 20.0, Self::Bat => 10.0, Self::Golem => 40.0 }
    }
    fn render_kind(self) -> u8 {
        match self { Self::Slime => 1, Self::Bat => 2, Self::Golem => 3 }
    }
    fn for_elapsed(elapsed_secs: f32, rng: &mut physics::rng::SimpleRng) -> Self {
        if elapsed_secs < 30.0 {
            Self::Slime
        } else if elapsed_secs < 60.0 {
            if rng.next_u32() % 2 == 0 { Self::Slime } else { Self::Bat }
        } else {
            match rng.next_u32() % 3 {
                0 => Self::Bat,
                1 => Self::Golem,
                _ => Self::Slime,
            }
        }
    }

}
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use physics::rng::SimpleRng;
use physics::separation::{apply_separation, EnemySeparation};
use physics::spatial_hash::CollisionWorld;

struct PlayerState {
    x: f32, y: f32,
    input_dx: f32, input_dy: f32,
    hp: f32,
    max_hp: f32,
    invincible_timer: f32,
}

struct EnemyWorld {
    positions_x:  Vec<f32>,
    positions_y:  Vec<f32>,
    hp:           Vec<f32>,
    alive:        Vec<bool>,
    kinds:        Vec<EnemyKind>,
    count:        usize,
    /// 分離パス用の作業バッファ（毎フレーム再利用してアロケーションを回避）
    sep_x:        Vec<f32>,
    sep_y:        Vec<f32>,
    /// 近隣クエリ結果の再利用バッファ（毎フレームのヒープアロケーションを回避）
    neighbor_buf: Vec<usize>,
}

impl EnemyWorld {
    fn new() -> Self {
        Self {
            positions_x:  Vec::new(),
            positions_y:  Vec::new(),
            hp:           Vec::new(),
            alive:        Vec::new(),
            kinds:        Vec::new(),
            count:        0,
            sep_x:        Vec::new(),
            sep_y:        Vec::new(),
            neighbor_buf: Vec::new(),
        }
    }
    fn spawn(&mut self, positions: &[(f32, f32)], kind: EnemyKind) {
        let max_hp = kind.max_hp();
        for &(x, y) in positions {
            let slot = self.alive.iter().position(|&a| !a);
            if let Some(i) = slot {
                self.positions_x[i] = x;
                self.positions_y[i] = y;
                self.hp[i]    = max_hp;
                self.alive[i] = true;
                self.kinds[i] = kind;
            } else {
                self.positions_x.push(x);
                self.positions_y.push(y);
                self.hp.push(max_hp);
                self.alive.push(true);
                self.kinds.push(kind);
                self.sep_x.push(0.0);
                self.sep_y.push(0.0);
            }
            self.count += 1;
        }
    }
    fn kill(&mut self, i: usize) {
        if self.alive[i] { self.alive[i] = false; self.count = self.count.saturating_sub(1); }
    }
    fn len(&self) -> usize { self.positions_x.len() }
}

impl EnemySeparation for EnemyWorld {
    fn enemy_count(&self) -> usize          { self.positions_x.len() }
    fn is_alive(&self, i: usize) -> bool    { self.alive[i] }
    fn pos_x(&self, i: usize) -> f32        { self.positions_x[i] }
    fn pos_y(&self, i: usize) -> f32        { self.positions_y[i] }
    fn add_pos_x(&mut self, i: usize, v: f32) { self.positions_x[i] += v; }
    fn add_pos_y(&mut self, i: usize, v: f32) { self.positions_y[i] += v; }
    fn sep_buf_x(&mut self) -> &mut Vec<f32>  { &mut self.sep_x }
    fn sep_buf_y(&mut self) -> &mut Vec<f32>  { &mut self.sep_y }
    fn neighbor_buf(&mut self) -> &mut Vec<usize> { &mut self.neighbor_buf }
}

struct BulletWorld {
    positions_x:  Vec<f32>,
    positions_y:  Vec<f32>,
    velocities_x: Vec<f32>,
    velocities_y: Vec<f32>,
    damage:       Vec<i32>,
    lifetime:     Vec<f32>,
    alive:        Vec<bool>,
    count:        usize,
}

impl BulletWorld {
    fn new() -> Self {
        Self { positions_x: Vec::new(), positions_y: Vec::new(), velocities_x: Vec::new(), velocities_y: Vec::new(), damage: Vec::new(), lifetime: Vec::new(), alive: Vec::new(), count: 0 }
    }
    fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32, dmg: i32) {
        let slot = self.alive.iter().position(|&a| !a);
        if let Some(i) = slot {
            self.positions_x[i]  = x;
            self.positions_y[i]  = y;
            self.velocities_x[i] = vx;
            self.velocities_y[i] = vy;
            self.damage[i]       = dmg;
            self.lifetime[i]     = BULLET_LIFETIME;
            self.alive[i]        = true;
        } else {
            self.positions_x.push(x);
            self.positions_y.push(y);
            self.velocities_x.push(vx);
            self.velocities_y.push(vy);
            self.damage.push(dmg);
            self.lifetime.push(BULLET_LIFETIME);
            self.alive.push(true);
        }
        self.count += 1;
    }
    fn kill(&mut self, i: usize) {
        if self.alive[i] { self.alive[i] = false; self.count = self.count.saturating_sub(1); }
    }
    fn len(&self) -> usize { self.positions_x.len() }
}

struct ParticleWorld {
    positions_x:  Vec<f32>,
    positions_y:  Vec<f32>,
    velocities_x: Vec<f32>,
    velocities_y: Vec<f32>,
    lifetime:     Vec<f32>,
    max_lifetime: Vec<f32>,
    color:        Vec<[f32; 4]>,
    size:         Vec<f32>,
    alive:        Vec<bool>,
    count:        usize,
    rng:          SimpleRng,
}

impl ParticleWorld {
    fn new(seed: u64) -> Self {
        Self {
            positions_x:  Vec::new(),
            positions_y:  Vec::new(),
            velocities_x: Vec::new(),
            velocities_y: Vec::new(),
            lifetime:     Vec::new(),
            max_lifetime: Vec::new(),
            color:        Vec::new(),
            size:         Vec::new(),
            alive:        Vec::new(),
            count:        0,
            rng:          SimpleRng::new(seed),
        }
    }

    fn len(&self) -> usize { self.positions_x.len() }

    fn spawn_one(&mut self, x: f32, y: f32, vx: f32, vy: f32, lt: f32, color: [f32; 4], size: f32) {
        for i in 0..self.positions_x.len() {
            if !self.alive[i] {
                self.positions_x[i]  = x;
                self.positions_y[i]  = y;
                self.velocities_x[i] = vx;
                self.velocities_y[i] = vy;
                self.lifetime[i]     = lt;
                self.max_lifetime[i] = lt;
                self.color[i]        = color;
                self.size[i]         = size;
                self.alive[i]        = true;
                self.count += 1;
                return;
            }
        }
        self.positions_x.push(x);
        self.positions_y.push(y);
        self.velocities_x.push(vx);
        self.velocities_y.push(vy);
        self.lifetime.push(lt);
        self.max_lifetime.push(lt);
        self.color.push(color);
        self.size.push(size);
        self.alive.push(true);
        self.count += 1;
    }

    fn emit(&mut self, x: f32, y: f32, count: usize, color: [f32; 4]) {
        for _ in 0..count {
            let angle = self.rng.next_f32() * std::f32::consts::TAU;
            let speed = 50.0 + self.rng.next_f32() * 150.0;
            let vx = angle.cos() * speed;
            let vy = angle.sin() * speed;
            let lt = 0.3 + self.rng.next_f32() * 0.4;
            let sz = 4.0 + self.rng.next_f32() * 4.0;
            self.spawn_one(x, y, vx, vy, lt, color, sz);
        }
    }

    fn kill(&mut self, i: usize) {
        if self.alive[i] { self.alive[i] = false; self.count = self.count.saturating_sub(1); }
    }
}

struct GameWorld {
    player:           PlayerState,
    enemies:          EnemyWorld,
    bullets:          BulletWorld,
    particles:        ParticleWorld,
    collision:        CollisionWorld,
    rng:              SimpleRng,
    score:            u32,
    elapsed_seconds:  f32,
    last_spawn_secs:  f32,
    // Step 17: 武器スロット（複数武器・レベル管理）
    weapon_slots:     Vec<WeaponSlot>,
    exp:              u32,
    level:            u32,
    level_up_pending: bool,
    weapon_choices:   Vec<String>,
}

impl GameWorld {
    fn new() -> Self {
        Self {
            player: PlayerState {
                x: SCREEN_WIDTH / 2.0 - PLAYER_SIZE / 2.0,
                y: SCREEN_HEIGHT / 2.0 - PLAYER_SIZE / 2.0,
                input_dx: 0.0, input_dy: 0.0,
                hp: 100.0, max_hp: 100.0,
                invincible_timer: 0.0,
            },
            enemies:          EnemyWorld::new(),
            bullets:          BulletWorld::new(),
            particles:        ParticleWorld::new(67890),
            collision:        CollisionWorld::new(CELL_SIZE),
            rng:              SimpleRng::new(42),
            score:            0,
            elapsed_seconds:  0.0,
            last_spawn_secs:  0.0,
            weapon_slots:     vec![WeaponSlot::new(WeaponKind::MagicWand)],
            exp:              0,
            level:            1,
            level_up_pending: false,
            weapon_choices:   Vec::new(),
        }
    }

    fn step(&mut self, dt: f32) {
        // レベルアップ中はゲームを一時停止（プレイヤーがボタンを選ぶまで待つ）
        if self.level_up_pending {
            return;
        }

        self.elapsed_seconds += dt;

        // プレイヤー移動
        let dx = self.player.input_dx;
        let dy = self.player.input_dy;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.001 {
            self.player.x += (dx / len) * PLAYER_SPEED * dt;
            self.player.y += (dy / len) * PLAYER_SPEED * dt;
        }
        self.player.x = self.player.x.clamp(0.0, SCREEN_WIDTH  - PLAYER_SIZE);
        self.player.y = self.player.y.clamp(0.0, SCREEN_HEIGHT - PLAYER_SIZE);

        let px = self.player.x + PLAYER_RADIUS;
        let py = self.player.y + PLAYER_RADIUS;

        // 敵 AI（EnemyKind ごとの速度を使用）
        for i in 0..self.enemies.len() {
            if !self.enemies.alive[i] { continue; }
            let ex = self.enemies.positions_x[i];
            let ey = self.enemies.positions_y[i];
            let ddx = px - ex;
            let ddy = py - ey;
            let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.001);
            let spd = self.enemies.kinds[i].speed();
            self.enemies.positions_x[i] += (ddx / dist) * spd * dt;
            self.enemies.positions_y[i] += (ddy / dist) * spd * dt;
        }

        // 敵同士の重なりを解消する分離パス
        apply_separation(&mut self.enemies, ENEMY_SEPARATION_RADIUS, ENEMY_SEPARATION_FORCE, dt);

        // 衝突: Spatial Hash 再構築
        self.collision.dynamic.clear();
        for i in 0..self.enemies.len() {
            if self.enemies.alive[i] {
                self.collision.dynamic.insert(i, self.enemies.positions_x[i], self.enemies.positions_y[i]);
            }
        }

        // 無敵タイマー
        if self.player.invincible_timer > 0.0 {
            self.player.invincible_timer = (self.player.invincible_timer - dt).max(0.0);
        }

        // パーティクル更新
        {
            let plen = self.particles.len();
            for i in 0..plen {
                if !self.particles.alive[i] { continue; }
                self.particles.positions_x[i] += self.particles.velocities_x[i] * dt;
                self.particles.positions_y[i] += self.particles.velocities_y[i] * dt;
                self.particles.velocities_y[i] += 200.0 * dt;
                self.particles.lifetime[i] -= dt;
                if self.particles.lifetime[i] <= 0.0 {
                    self.particles.kill(i);
                }
            }
        }

        // プレイヤー vs 敵（EnemyKind ごとの半径・ダメージを使用）
        let max_enemy_r = 32.0_f32;
        let query_r = PLAYER_RADIUS + max_enemy_r;
        let candidates = self.collision.dynamic.query_nearby(px, py, query_r);
        for idx in candidates {
            if !self.enemies.alive[idx] { continue; }
            let kind    = self.enemies.kinds[idx];
            let enemy_r = kind.radius();
            let hit_r   = PLAYER_RADIUS + enemy_r;
            let ex = self.enemies.positions_x[idx] + enemy_r;
            let ey = self.enemies.positions_y[idx] + enemy_r;
            let ddx = px - ex;
            let ddy = py - ey;
            if ddx * ddx + ddy * ddy < hit_r * hit_r {
                if self.player.invincible_timer <= 0.0 && self.player.hp > 0.0 {
                    self.player.hp = (self.player.hp - kind.damage_per_sec() * dt).max(0.0);
                    self.player.invincible_timer = INVINCIBLE_DURATION;
                    // 赤いパーティクル
                    self.particles.emit(px, py, 6, [1.0, 0.15, 0.15, 1.0]);
                }
            }
        }

        // Step 17: 武器スロット発射処理（レベルに応じたクールダウン・ダメージ・弾数）
        let slot_count = self.weapon_slots.len();
        for si in 0..slot_count {
            self.weapon_slots[si].cooldown_timer = (self.weapon_slots[si].cooldown_timer - dt).max(0.0);
            if self.weapon_slots[si].cooldown_timer > 0.0 { continue; }

            let cd     = self.weapon_slots[si].effective_cooldown();
            let dmg    = self.weapon_slots[si].effective_damage();
            let bcount = self.weapon_slots[si].bullet_count();

            match self.weapon_slots[si].kind {
                WeaponKind::MagicWand => {
                    if let Some(ti) = self.find_nearest_enemy(px, py) {
                        let target_r = self.enemies.kinds[ti].radius();
                        let tx  = self.enemies.positions_x[ti] + target_r;
                        let ty  = self.enemies.positions_y[ti] + target_r;
                        let bdx = tx - px;
                        let bdy = ty - py;
                        let base_angle = bdy.atan2(bdx);
                        let spread = std::f32::consts::PI * 0.08;
                        let half   = (bcount as f32 - 1.0) / 2.0;
                        for bi in 0..bcount {
                            let angle = base_angle + (bi as f32 - half) * spread;
                            self.bullets.spawn(px, py, angle.cos() * BULLET_SPEED, angle.sin() * BULLET_SPEED, dmg);
                        }
                        self.weapon_slots[si].cooldown_timer = cd;
                    }
                }
                WeaponKind::Axe => {
                    self.bullets.spawn(px, py, 0.0, -BULLET_SPEED, dmg);
                    self.weapon_slots[si].cooldown_timer = cd;
                }
                WeaponKind::Cross => {
                    let dirs_4: [(f32, f32); 4] = [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)];
                    let diag = std::f32::consts::FRAC_1_SQRT_2;
                    let dirs_8: [(f32, f32); 8] = [
                        (0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0),
                        (diag, -diag), (-diag, -diag), (diag, diag), (-diag, diag),
                    ];
                    let dirs: &[(f32, f32)] = if bcount >= 8 { &dirs_8 } else { &dirs_4 };
                    for &(dx_dir, dy_dir) in dirs {
                        self.bullets.spawn(px, py, dx_dir * BULLET_SPEED, dy_dir * BULLET_SPEED, dmg);
                    }
                    self.weapon_slots[si].cooldown_timer = cd;
                }
            }
        }

        // Bullet movement and lifetime
        let bl = self.bullets.len();
        for i in 0..bl {
            if !self.bullets.alive[i] { continue; }
            self.bullets.positions_x[i] += self.bullets.velocities_x[i] * dt;
            self.bullets.positions_y[i] += self.bullets.velocities_y[i] * dt;
            self.bullets.lifetime[i]    -= dt;
            if self.bullets.lifetime[i] <= 0.0 {
                self.bullets.kill(i);
                continue;
            }
            let bx = self.bullets.positions_x[i];
            let by = self.bullets.positions_y[i];
            if bx < -100.0 || bx > SCREEN_WIDTH + 100.0 || by < -100.0 || by > SCREEN_HEIGHT + 100.0 {
                self.bullets.kill(i);
            }
        }

        // Bullet vs enemy collision（EnemyKind ごとの半径・経験値を使用）
        let bullet_query_r = BULLET_RADIUS + 32.0_f32;
        for bi in 0..bl {
            if !self.bullets.alive[bi] { continue; }
            let bx  = self.bullets.positions_x[bi];
            let by  = self.bullets.positions_y[bi];
            let dmg = self.bullets.damage[bi];
            let nearby = self.collision.dynamic.query_nearby(bx, by, bullet_query_r);
            for ei in nearby {
                if !self.enemies.alive[ei] { continue; }
                let kind    = self.enemies.kinds[ei];
                let enemy_r = kind.radius();
                let hit_r   = BULLET_RADIUS + enemy_r;
                let ex  = self.enemies.positions_x[ei] + enemy_r;
                let ey  = self.enemies.positions_y[ei] + enemy_r;
                let ddx = bx - ex;
                let ddy = by - ey;
                if ddx * ddx + ddy * ddy < hit_r * hit_r {
                    self.enemies.hp[ei] -= dmg as f32;
                    if self.enemies.hp[ei] <= 0.0 {
                        self.enemies.kill(ei);
                        self.score += kind.exp_reward() * 2;
                        self.exp   += kind.exp_reward();
                        self.check_level_up();
                        // 撃破: タイプ別パーティクル
                        let pc = match kind {
                            EnemyKind::Slime => [1.0, 0.5, 0.1, 1.0],
                            EnemyKind::Bat   => [0.7, 0.2, 0.9, 1.0],
                            EnemyKind::Golem => [0.6, 0.6, 0.6, 1.0],
                        };
                        self.particles.emit(ex, ey, 8, pc);
                    } else {
                        // ヒット: 黄色パーティクル
                        self.particles.emit(ex, ey, 3, [1.0, 0.9, 0.3, 1.0]);
                    }
                    self.bullets.kill(bi);
                    break;
                }
            }
        }

        // Wave-based enemy spawn（Step 18: タイプ別スポーン）
        let (wave_interval, wave_count) = current_wave(self.elapsed_seconds);
        if self.elapsed_seconds - self.last_spawn_secs >= wave_interval
            && self.enemies.count < MAX_ENEMIES
        {
            let to_spawn = wave_count.min(MAX_ENEMIES - self.enemies.count);
            let kind = EnemyKind::for_elapsed(self.elapsed_seconds, &mut self.rng);
            let positions: Vec<(f32, f32)> = (0..to_spawn)
                .map(|_| spawn_outside(&mut self.rng))
                .collect();
            self.enemies.spawn(&positions, kind);
            self.last_spawn_secs = self.elapsed_seconds;
        }
    }

    fn find_nearest_enemy(&self, px: f32, py: f32) -> Option<usize> {
        let mut min_d = f32::MAX;
        let mut nearest = None;
        for i in 0..self.enemies.len() {
            if !self.enemies.alive[i] { continue; }
            let dx = self.enemies.positions_x[i] - px;
            let dy = self.enemies.positions_y[i] - py;
            let d  = dx * dx + dy * dy;
            if d < min_d { min_d = d; nearest = Some(i); }
        }
        nearest
    }

    fn check_level_up(&mut self) {
        if self.level_up_pending { return; }
        let required = exp_for_next(self.level);
        if self.exp >= required {
            self.level_up_pending = true;
            // 選択肢: 未所持優先 → 低レベル順（Lv.8 は除外）
            let all: &[(&str, WeaponKind)] = &[
                ("magic_wand", WeaponKind::MagicWand),
                ("axe",        WeaponKind::Axe),
                ("cross",      WeaponKind::Cross),
            ];
            let mut choices: Vec<(i32, String)> = all.iter()
                .filter_map(|(name, kind)| {
                    let lv = self.weapon_slots.iter()
                        .find(|s| &s.kind == kind)
                        .map(|s| s.level)
                        .unwrap_or(0);
                    if lv >= 8 { return None; }
                    let sort_key = if lv == 0 { -1i32 } else { lv as i32 };
                    Some((sort_key, name.to_string()))
                })
                .collect();
            choices.sort_by_key(|(k, _)| *k);
            self.weapon_choices = choices.into_iter().take(3).map(|(_, n)| n).collect();
        }
    }

    /// 武器を選択してレベルアップを確定する
    fn select_weapon(&mut self, weapon_name: &str) {
        if weapon_name != "__skip__" {
            let kind = match weapon_name {
                "axe"   => WeaponKind::Axe,
                "cross" => WeaponKind::Cross,
                _       => WeaponKind::MagicWand,
            };
            if let Some(slot) = self.weapon_slots.iter_mut().find(|s| s.kind == kind) {
                slot.level = (slot.level + 1).min(MAX_WEAPON_LEVEL);
            } else if self.weapon_slots.len() < MAX_WEAPON_SLOTS {
                self.weapon_slots.push(WeaponSlot::new(kind));
            }
        }
        self.level            += 1;
        self.level_up_pending  = false;
        self.weapon_choices.clear();
    }

    fn get_render_data(&self) -> Vec<(f32, f32, u8)> {
        let mut v = Vec::with_capacity(1 + self.enemies.len() + self.bullets.len());
        v.push((self.player.x, self.player.y, 0u8));
        for i in 0..self.enemies.len() {
            if self.enemies.alive[i] {
                v.push((
                    self.enemies.positions_x[i],
                    self.enemies.positions_y[i],
                    self.enemies.kinds[i].render_kind(),
                ));
            }
        }
        for i in 0..self.bullets.len() {
            if self.bullets.alive[i] {
                v.push((self.bullets.positions_x[i], self.bullets.positions_y[i], 4u8));
            }
        }
        v
    }

    fn get_particle_data(&self) -> Vec<(f32, f32, f32, f32, f32, f32, f32)> {
        let mut v = Vec::with_capacity(self.particles.count);
        for i in 0..self.particles.len() {
            if !self.particles.alive[i] { continue; }
            let alpha = (self.particles.lifetime[i] / self.particles.max_lifetime[i]).clamp(0.0, 1.0);
            let c = self.particles.color[i];
            v.push((
                self.particles.positions_x[i],
                self.particles.positions_y[i],
                c[0], c[1], c[2], alpha,
                self.particles.size[i],
            ));
        }
        v
    }

    fn hud_data(&self, fps: f32) -> HudData {
        HudData {
            hp:              self.player.hp,
            max_hp:          self.player.max_hp,
            score:           self.score,
            elapsed_seconds: self.elapsed_seconds,
            level:           self.level,
            exp:             self.exp,
            exp_to_next:     exp_for_next(self.level).saturating_sub(self.exp),
            enemy_count:     self.enemies.count,
            bullet_count:    self.bullets.count,
            fps,
            level_up_pending: self.level_up_pending,
            weapon_choices:  self.weapon_choices.clone(),
            // Step 17: 武器レベル情報
            weapon_levels:   self.weapon_slots.iter()
                .map(|s| (s.kind.name().to_string(), s.level))
                .collect(),
        }
    }
}

fn current_wave(elapsed_secs: f32) -> (f32, usize) {
    WAVES.iter()
        .filter(|&&(start, _, _)| elapsed_secs >= start)
        .last()
        .map(|&(_, interval, count)| (interval, count))
        .unwrap_or((0.8, 20))
}

fn exp_for_next(level: u32) -> u32 {
    const TABLE: [u32; 10] = [0, 10, 25, 45, 70, 100, 135, 175, 220, 270];
    let idx = level as usize;
    if idx < TABLE.len() { TABLE[idx] } else { 270 + (idx as u32 - 9) * 50 }
}

fn spawn_outside(rng: &mut SimpleRng) -> (f32, f32) {
    let margin = 80.0;
    match rng.next_u32() % 4 {
        0 => (rng.next_f32() * SCREEN_WIDTH, -margin),
        1 => (rng.next_f32() * SCREEN_WIDTH, SCREEN_HEIGHT + margin),
        2 => (-margin,                        rng.next_f32() * SCREEN_HEIGHT),
        _ => (SCREEN_WIDTH + margin,          rng.next_f32() * SCREEN_HEIGHT),
    }
}

// ─── winit application ────────────────────────────────────────

struct App {
    window:      Option<Arc<Window>>,
    renderer:    Option<Renderer>,
    game:        GameWorld,
    keys_held:   HashSet<KeyCode>,
    last_update: Option<Instant>,
}

impl App {
    fn new() -> Self {
        Self {
            window:      None,
            renderer:    None,
            game:        GameWorld::new(),
            keys_held:   HashSet::new(),
            last_update: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Elixir x Rust Survivor")
                        .with_inner_size(winit::dpi::LogicalSize::new(
                            SCREEN_WIDTH as u32,
                            SCREEN_HEIGHT as u32,
                        )),
                )
                .expect("ウィンドウの作成に失敗しました"),
        );

        let renderer = pollster::block_on(Renderer::new(window.clone()));

        self.window      = Some(window);
        self.renderer    = Some(renderer);
        self.last_update = Some(Instant::now());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // egui にイベントを転送（消費された場合はゲームへ渡さない）
        if let (Some(renderer), Some(window)) = (self.renderer.as_mut(), self.window.as_ref()) {
            if renderer.handle_window_event(window, &event) {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
            }

            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state,
                    ..
                },
                ..
            } => {
                match state {
                    ElementState::Pressed  => { self.keys_held.insert(code); }
                    ElementState::Released => { self.keys_held.remove(&code); }
                }
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();

                // ─── 入力処理 ──────────────────────────────────
                let mut dx = 0.0f32;
                let mut dy = 0.0f32;
                if self.keys_held.contains(&KeyCode::KeyW) || self.keys_held.contains(&KeyCode::ArrowUp)    { dy -= 1.0; }
                if self.keys_held.contains(&KeyCode::KeyS) || self.keys_held.contains(&KeyCode::ArrowDown)  { dy += 1.0; }
                if self.keys_held.contains(&KeyCode::KeyA) || self.keys_held.contains(&KeyCode::ArrowLeft)  { dx -= 1.0; }
                if self.keys_held.contains(&KeyCode::KeyD) || self.keys_held.contains(&KeyCode::ArrowRight) { dx += 1.0; }
                self.game.player.input_dx = dx;
                self.game.player.input_dy = dy;

                // Step 17: レベルアップ中は 1/2/3 キーで武器選択、Esc でスキップ
                if self.game.level_up_pending {
                    if self.keys_held.contains(&KeyCode::Escape) {
                        self.keys_held.remove(&KeyCode::Escape);
                        self.game.select_weapon("__skip__");
                    } else {
                        let idx = if self.keys_held.contains(&KeyCode::Digit1) { Some(0) }
                                  else if self.keys_held.contains(&KeyCode::Digit2) { Some(1) }
                                  else if self.keys_held.contains(&KeyCode::Digit3) { Some(2) }
                                  else { None };
                        if let Some(i) = idx {
                            if let Some(choice) = self.game.weapon_choices.get(i).cloned() {
                                // キーを離すまで連続選択しないよう、選択後にキーを消費
                                self.keys_held.remove(&KeyCode::Digit1);
                                self.keys_held.remove(&KeyCode::Digit2);
                                self.keys_held.remove(&KeyCode::Digit3);
                                self.game.select_weapon(&choice);
                            }
                        }
                    }
                }

                // ─── ゲームステップ ────────────────────────────
                if let Some(last) = self.last_update {
                    let dt = now.duration_since(last).as_secs_f32().min(0.05);
                    self.game.step(dt);
                }
                self.last_update = Some(now);

                // ─── 描画 ──────────────────────────────────────
                if let (Some(renderer), Some(window)) =
                    (self.renderer.as_mut(), self.window.as_ref())
                {
                    let render_data   = self.game.get_render_data();
                    let particle_data = self.game.get_particle_data();
                    renderer.update_instances(&render_data, &particle_data);
                    let hud = self.game.hud_data(renderer.current_fps);
                    // Step 17: ボタンクリックで武器選択（"__skip__" はスキップ扱い）
                    if let Some(chosen) = renderer.render(window, &hud) {
                        self.game.select_weapon(&chosen);
                    }
                }

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
