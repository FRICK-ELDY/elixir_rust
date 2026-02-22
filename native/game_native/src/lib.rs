mod constants;
mod physics;

use constants::{
    BULLET_DAMAGE, BULLET_LIFETIME, BULLET_RADIUS, BULLET_SPEED,
    CELL_SIZE, ENEMY_DAMAGE_PER_SEC, ENEMY_RADIUS, ENEMY_SEPARATION_FORCE,
    ENEMY_SEPARATION_RADIUS, FRAME_BUDGET_MS,
    INVINCIBLE_DURATION, PLAYER_RADIUS, PLAYER_SIZE, PLAYER_SPEED,
    SCREEN_HEIGHT, SCREEN_WIDTH, WEAPON_COOLDOWN,
};
use physics::rng::SimpleRng;
use physics::separation::{apply_separation, EnemySeparation};
use physics::spatial_hash::CollisionWorld;
use rayon::prelude::*;
use rustler::{Atom, NifResult, ResourceArc};
use std::sync::Mutex;

rustler::atoms! {
    ok,
    slime,
    // 武器種別アトム
    magic_wand,
    axe,
    cross,
    // level_up 通知アトム
    level_up,
    no_change,
}

// ─── Player ───────────────────────────────────────────────────
pub struct PlayerState {
    pub x:                f32,
    pub y:                f32,
    pub input_dx:         f32,
    pub input_dy:         f32,
    pub hp:               f32,
    pub invincible_timer: f32,
}

// ─── 敵 SoA ──────────────────────────────────────────────────
pub struct EnemyWorld {
    pub positions_x:  Vec<f32>,
    pub positions_y:  Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub speeds:       Vec<f32>,
    pub hp:           Vec<f32>,
    pub alive:        Vec<bool>,
    pub count:        usize,
    /// 分離パス用の作業バッファ（毎フレーム再利用してアロケーションを回避）
    pub sep_x:        Vec<f32>,
    pub sep_y:        Vec<f32>,
    /// 近隣クエリ結果の再利用バッファ（毎フレームのヒープアロケーションを回避）
    pub neighbor_buf: Vec<usize>,
}

impl EnemyWorld {
    pub fn new() -> Self {
        Self {
            positions_x:  Vec::new(),
            positions_y:  Vec::new(),
            velocities_x: Vec::new(),
            velocities_y: Vec::new(),
            speeds:       Vec::new(),
            hp:           Vec::new(),
            alive:        Vec::new(),
            count:        0,
            sep_x:        Vec::new(),
            sep_y:        Vec::new(),
            neighbor_buf: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.positions_x.len()
    }

    pub fn kill(&mut self, i: usize) {
        if self.alive[i] {
            self.alive[i] = false;
            self.count = self.count.saturating_sub(1);
        }
    }

    /// 画面外のランダムな位置に `n` 体スポーン（死んだスロットを再利用）
    pub fn spawn(&mut self, positions: &[(f32, f32)]) {
        let mut slot = 0usize;
        for &(x, y) in positions {
            // 死んでいるスロットを探して再利用
            let reused = loop {
                if slot >= self.positions_x.len() {
                    break false;
                }
                if !self.alive[slot] {
                    break true;
                }
                slot += 1;
            };

            if reused {
                self.positions_x[slot]  = x;
                self.positions_y[slot]  = y;
                self.velocities_x[slot] = 0.0;
                self.velocities_y[slot] = 0.0;
                self.speeds[slot]       = 80.0;
                self.hp[slot]           = 30.0;
                self.alive[slot]        = true;
                self.count += 1;
                slot += 1;
            } else {
                self.positions_x.push(x);
                self.positions_y.push(y);
                self.velocities_x.push(0.0);
                self.velocities_y.push(0.0);
                self.speeds.push(80.0);
                self.hp.push(30.0);
                self.alive.push(true);
                self.sep_x.push(0.0);
                self.sep_y.push(0.0);
                self.count += 1;
            }
        }
    }
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

/// 画面外の四辺いずれかにランダムに配置
fn spawn_position_outside(rng: &mut SimpleRng, sw: f32, sh: f32) -> (f32, f32) {
    let margin = 80.0;
    let side = rng.next_u32() % 4;
    match side {
        0 => (rng.next_f32() * sw, -margin),
        1 => (rng.next_f32() * sw, sh + margin),
        2 => (-margin,             rng.next_f32() * sh),
        _ => (sw + margin,         rng.next_f32() * sh),
    }
}

// ─── 弾丸 SoA ─────────────────────────────────────────────────
pub struct BulletWorld {
    pub positions_x:  Vec<f32>,
    pub positions_y:  Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub damage:       Vec<i32>,
    pub lifetime:     Vec<f32>,
    pub alive:        Vec<bool>,
    pub count:        usize,
}

impl BulletWorld {
    pub fn new() -> Self {
        Self {
            positions_x:  Vec::new(),
            positions_y:  Vec::new(),
            velocities_x: Vec::new(),
            velocities_y: Vec::new(),
            damage:       Vec::new(),
            lifetime:     Vec::new(),
            alive:        Vec::new(),
            count:        0,
        }
    }

    pub fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32, damage: i32, lifetime: f32) {
        // 死んでいるスロットを再利用
        for i in 0..self.positions_x.len() {
            if !self.alive[i] {
                self.positions_x[i]  = x;
                self.positions_y[i]  = y;
                self.velocities_x[i] = vx;
                self.velocities_y[i] = vy;
                self.damage[i]       = damage;
                self.lifetime[i]     = lifetime;
                self.alive[i]        = true;
                self.count += 1;
                return;
            }
        }
        self.positions_x.push(x);
        self.positions_y.push(y);
        self.velocities_x.push(vx);
        self.velocities_y.push(vy);
        self.damage.push(damage);
        self.lifetime.push(lifetime);
        self.alive.push(true);
        self.count += 1;
    }

    pub fn kill(&mut self, i: usize) {
        if self.alive[i] {
            self.alive[i] = false;
            self.count = self.count.saturating_sub(1);
        }
    }

    pub fn len(&self) -> usize {
        self.positions_x.len()
    }
}

// ─── 武器種別 ─────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WeaponKind {
    /// 基本武器: 最近接敵に向けて弾丸を発射
    MagicWand,
    /// 斧: プレイヤーの上方向に放物線を描いて飛ぶ（簡易実装: 上方向に直進）
    Axe,
    /// 十字: 上下左右 4 方向に同時発射
    Cross,
}

impl WeaponKind {
    /// 武器ごとの発射クールダウン（秒）
    pub fn cooldown(&self) -> f32 {
        match self {
            WeaponKind::MagicWand => WEAPON_COOLDOWN,
            WeaponKind::Axe       => 1.5,
            WeaponKind::Cross     => 2.0,
        }
    }
    /// 武器ごとのダメージ
    pub fn damage(&self) -> i32 {
        match self {
            WeaponKind::MagicWand => BULLET_DAMAGE,
            WeaponKind::Axe       => 25,
            WeaponKind::Cross     => 15,
        }
    }
}

// ─── 武器スロット ─────────────────────────────────────────────
pub struct WeaponSlot {
    pub kind:           WeaponKind,
    pub level:          u32,   // 1〜8
    pub cooldown_timer: f32,
}

impl WeaponSlot {
    pub fn new(kind: WeaponKind) -> Self {
        Self { kind, level: 1, cooldown_timer: 0.0 }
    }

    /// レベルに応じたクールダウン（レベルが上がるほど速く撃てる）
    pub fn effective_cooldown(&self) -> f32 {
        let base = self.kind.cooldown();
        // Lv1 が基準、Lv8 で 50% 短縮
        (base * (1.0 - (self.level as f32 - 1.0) * 0.07)).max(base * 0.5)
    }

    /// レベルに応じたダメージ（レベルが上がるほど強い）
    pub fn effective_damage(&self) -> i32 {
        let base = self.kind.damage();
        base + (self.level as i32 - 1) * (base / 4).max(1)
    }

    /// レベルに応じた弾丸数
    pub fn bullet_count(&self) -> usize {
        match self.kind {
            WeaponKind::MagicWand => match self.level {
                1..=2 => 1,
                3..=4 => 2,
                5..=6 => 3,
                _     => 4,
            },
            WeaponKind::Cross => match self.level {
                1..=3 => 4,  // 上下左右
                _     => 8,  // 斜め追加
            },
            _ => 1,
        }
    }
}


// ─── パーティクル SoA ──────────────────────────────────────────
pub struct ParticleWorld {
    pub positions_x:  Vec<f32>,
    pub positions_y:  Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub lifetime:     Vec<f32>,
    pub max_lifetime: Vec<f32>,
    pub color:        Vec<[f32; 4]>,
    pub size:         Vec<f32>,
    pub alive:        Vec<bool>,
    pub count:        usize,
    rng:              SimpleRng,
}

impl ParticleWorld {
    pub fn new(seed: u64) -> Self {
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

    pub fn len(&self) -> usize {
        self.positions_x.len()
    }

    pub fn spawn_one(
        &mut self,
        x: f32, y: f32,
        vx: f32, vy: f32,
        lifetime: f32,
        color: [f32; 4],
        size: f32,
    ) {
        for i in 0..self.positions_x.len() {
            if !self.alive[i] {
                self.positions_x[i]  = x;
                self.positions_y[i]  = y;
                self.velocities_x[i] = vx;
                self.velocities_y[i] = vy;
                self.lifetime[i]     = lifetime;
                self.max_lifetime[i] = lifetime;
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
        self.lifetime.push(lifetime);
        self.max_lifetime.push(lifetime);
        self.color.push(color);
        self.size.push(size);
        self.alive.push(true);
        self.count += 1;
    }

    pub fn emit(&mut self, x: f32, y: f32, count: usize, color: [f32; 4]) {
        for _ in 0..count {
            let angle = self.rng.next_f32() * std::f32::consts::TAU;
            let speed = 50.0 + self.rng.next_f32() * 150.0;
            let vx = angle.cos() * speed;
            let vy = angle.sin() * speed;
            let lifetime = 0.3 + self.rng.next_f32() * 0.4;
            let size = 4.0 + self.rng.next_f32() * 4.0;
            self.spawn_one(x, y, vx, vy, lifetime, color, size);
        }
    }

    pub fn kill(&mut self, i: usize) {
        if self.alive[i] {
            self.alive[i] = false;
            self.count = self.count.saturating_sub(1);
        }
    }
}

/// 最近接の生存敵インデックスを返す
pub fn find_nearest_enemy(enemies: &EnemyWorld, px: f32, py: f32) -> Option<usize> {
    let mut min_dist = f32::MAX;
    let mut nearest  = None;
    for i in 0..enemies.len() {
        if !enemies.alive[i] {
            continue;
        }
        let dx   = enemies.positions_x[i] - px;
        let dy   = enemies.positions_y[i] - py;
        let dist = dx * dx + dy * dy;
        if dist < min_dist {
            min_dist = dist;
            nearest  = Some(i);
        }
    }
    nearest
}

/// Chase AI: 全敵をプレイヤーに向けて移動（rayon で並列化）
pub fn update_chase_ai(enemies: &mut EnemyWorld, player_x: f32, player_y: f32, dt: f32) {
    let len = enemies.len();
    // 各 SoA 配列をスライスとして取り出し、zip で並列イテレート
    let positions_x  = &mut enemies.positions_x[..len];
    let positions_y  = &mut enemies.positions_y[..len];
    let velocities_x = &mut enemies.velocities_x[..len];
    let velocities_y = &mut enemies.velocities_y[..len];
    let speeds       = &enemies.speeds[..len];
    let alive        = &enemies.alive[..len];

    (
        positions_x,
        positions_y,
        velocities_x,
        velocities_y,
        speeds,
        alive,
    )
        .into_par_iter()
        .for_each(|(px, py, vx, vy, speed, is_alive)| {
            if !*is_alive {
                return;
            }
            let dx   = player_x - *px;
            let dy   = player_y - *py;
            let dist = (dx * dx + dy * dy).sqrt().max(0.001);
            *vx  = (dx / dist) * speed;
            *vy  = (dy / dist) * speed;
            *px += *vx * dt;
            *py += *vy * dt;
        });
}


// ─── ゲームワールド ───────────────────────────────────────────
pub struct GameWorldInner {
    pub frame_id:           u32,
    pub player:             PlayerState,
    pub enemies:            EnemyWorld,
    pub bullets:            BulletWorld,
    pub particles:          ParticleWorld,
    pub rng:                SimpleRng,
    pub collision:          CollisionWorld,
    /// 直近フレームの物理ステップ処理時間（ミリ秒）
    pub last_frame_time_ms: f64,
    /// ─── Step 13: HUD ─────────────────────────────────────────
    /// 撃破スコア（敵 1 体 = 10 点）
    pub score:              u32,
    /// ゲーム開始からの経過時間（秒）
    pub elapsed_seconds:    f32,
    /// プレイヤーの最大 HP（HP バー計算用）
    pub player_max_hp:      f32,
    /// ─── Step 14: レベルアップ ────────────────────────────────
    /// 現在の経験値
    pub exp:                u32,
    /// 現在のレベル（1 始まり）
    pub level:              u32,
    /// レベルアップ待機フラグ（Elixir 側が武器選択を完了するまで true）
    pub level_up_pending:   bool,
    /// 装備中の武器スロット（最大 6 つ）
    pub weapon_slots:       Vec<WeaponSlot>,
}

impl GameWorldInner {
    /// 衝突判定用の Spatial Hash を再構築する（clone 不要）
    fn rebuild_collision(&mut self) {
        self.collision.dynamic.clear();
        self.enemies.alive
            .iter()
            .enumerate()
            .filter(|&(_, &is_alive)| is_alive)
            .for_each(|(i, _)| {
                self.collision.dynamic.insert(
                    i,
                    self.enemies.positions_x[i],
                    self.enemies.positions_y[i],
                );
            });
    }
}

pub struct GameWorld(pub Mutex<GameWorldInner>);

// ─── NIF 関数 ─────────────────────────────────────────────────

#[rustler::nif]
fn add(a: i64, b: i64) -> NifResult<i64> {
    Ok(a + b)
}

#[rustler::nif]
fn create_world() -> ResourceArc<GameWorld> {
    ResourceArc::new(GameWorld(Mutex::new(GameWorldInner {
        frame_id:           0,
        player:             PlayerState {
            x:                SCREEN_WIDTH  / 2.0 - PLAYER_SIZE / 2.0,
            y:                SCREEN_HEIGHT / 2.0 - PLAYER_SIZE / 2.0,
            input_dx:         0.0,
            input_dy:         0.0,
            hp:               100.0,
            invincible_timer: 0.0,
        },
        enemies:            EnemyWorld::new(),
        bullets:            BulletWorld::new(),
        particles:          ParticleWorld::new(67890),
        rng:                SimpleRng::new(12345),
        collision:          CollisionWorld::new(CELL_SIZE),
        last_frame_time_ms: 0.0,
        score:              0,
        elapsed_seconds:    0.0,
        player_max_hp:      100.0,
        exp:                0,
        level:              1,
        level_up_pending:   false,
        weapon_slots:       vec![WeaponSlot::new(WeaponKind::MagicWand)],
    })))
}

/// プレイヤーの入力方向を設定（Step 8）
#[rustler::nif]
fn set_player_input(world: ResourceArc<GameWorld>, dx: f64, dy: f64) -> Atom {
    let mut w = world.0.lock().unwrap();
    w.player.input_dx = dx as f32;
    w.player.input_dy = dy as f32;
    ok()
}

/// 敵をスポーン（Step 9）
#[rustler::nif]
fn spawn_enemies(world: ResourceArc<GameWorld>, _kind: Atom, count: usize) -> Atom {
    let mut w = world.0.lock().unwrap();
    // rng の借用を先に終わらせてから enemies に渡す
    let positions: Vec<(f32, f32)> = (0..count)
        .map(|_| spawn_position_outside(&mut w.rng, SCREEN_WIDTH, SCREEN_HEIGHT))
        .collect();
    w.enemies.spawn(&positions);
    ok()
}

/// 物理ステップ: プレイヤー移動 + Chase AI + 衝突判定（Step 8/9/10/12）
#[rustler::nif(schedule = "DirtyCpu")]
fn physics_step(world: ResourceArc<GameWorld>, delta_ms: f64) -> u32 {
    let t_start = std::time::Instant::now();

    let mut w = world.0.lock().unwrap();
    w.frame_id += 1;

    let dt = delta_ms as f32 / 1000.0;

    // ── Step 13: 経過時間を更新 ──────────────────────────────────
    w.elapsed_seconds += dt;
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

    // Chase AI
    let px = w.player.x + PLAYER_RADIUS;
    let py = w.player.y + PLAYER_RADIUS;
    update_chase_ai(&mut w.enemies, px, py, dt);

    // 敵同士の重なりを解消する分離パス
    apply_separation(&mut w.enemies, ENEMY_SEPARATION_RADIUS, ENEMY_SEPARATION_FORCE, dt);

    // ── Step 10: 衝突判定（Spatial Hash）────────────────────────
    // 1. 動的 Spatial Hash を再構築
    w.rebuild_collision();

    // 2. プレイヤー周辺の敵を取得して円-円判定
    let hit_radius = PLAYER_RADIUS + ENEMY_RADIUS;
    let candidates = w.collision.dynamic.query_nearby(px, py, hit_radius);

    // 無敵タイマーを更新
    if w.player.invincible_timer > 0.0 {
        w.player.invincible_timer = (w.player.invincible_timer - dt).max(0.0);
    }

    for idx in candidates {
        if !w.enemies.alive[idx] {
            continue;
        }
        let ex = w.enemies.positions_x[idx] + ENEMY_RADIUS;
        let ey = w.enemies.positions_y[idx] + ENEMY_RADIUS;
        let ddx = px - ex;
        let ddy = py - ey;
        let dist_sq = ddx * ddx + ddy * ddy;

        if dist_sq < hit_radius * hit_radius {
            // 敵→プレイヤーへのダメージ（無敵時間中は無効）
            if w.player.invincible_timer <= 0.0 && w.player.hp > 0.0 {
                w.player.hp = (w.player.hp - ENEMY_DAMAGE_PER_SEC * dt).max(0.0);
                w.player.invincible_timer = INVINCIBLE_DURATION;
                // 赤いパーティクルをプレイヤー位置に発生
                let ppx = w.player.x + PLAYER_RADIUS;
                let ppy = w.player.y + PLAYER_RADIUS;
                w.particles.emit(ppx, ppy, 6, [1.0, 0.15, 0.15, 1.0]);
            }
        }
    }

    // ── Step 11/14/17: 武器スロット発射処理 ─────────────────────
    // level_up_pending 中は発射を止めてゲームを一時停止する
    if !w.level_up_pending {
        let slot_count = w.weapon_slots.len();
        for si in 0..slot_count {
            w.weapon_slots[si].cooldown_timer = (w.weapon_slots[si].cooldown_timer - dt).max(0.0);
            if w.weapon_slots[si].cooldown_timer > 0.0 {
                continue;
            }
            let kind  = w.weapon_slots[si].kind;
            // Step 17: レベルに応じたクールダウン・ダメージ・弾数を使用
            let cd    = w.weapon_slots[si].effective_cooldown();
            let dmg   = w.weapon_slots[si].effective_damage();
            let bcount = w.weapon_slots[si].bullet_count();
            match kind {
                WeaponKind::MagicWand => {
                    if let Some(ti) = find_nearest_enemy(&w.enemies, px, py) {
                        let tx   = w.enemies.positions_x[ti] + ENEMY_RADIUS;
                        let ty   = w.enemies.positions_y[ti] + ENEMY_RADIUS;
                        let bdx  = tx - px;
                        let bdy  = ty - py;
                        // bcount 発同時発射（Lv3 で 2 発、Lv5 で 3 発）
                        // 複数発は少しずつ角度をずらして扇状に発射
                        let base_angle = bdy.atan2(bdx);
                        let spread = std::f32::consts::PI * 0.08; // 約 14 度の広がり
                        let half = (bcount as f32 - 1.0) / 2.0;
                        for bi in 0..bcount {
                            let angle = base_angle + (bi as f32 - half) * spread;
                            let vx = angle.cos() * BULLET_SPEED;
                            let vy = angle.sin() * BULLET_SPEED;
                            w.bullets.spawn(px, py, vx, vy, dmg, BULLET_LIFETIME);
                        }
                        w.weapon_slots[si].cooldown_timer = cd;
                    }
                }
                WeaponKind::Axe => {
                    // 上方向に直進（簡易実装）
                    w.bullets.spawn(px, py, 0.0, -BULLET_SPEED, dmg, BULLET_LIFETIME);
                    w.weapon_slots[si].cooldown_timer = cd;
                }
                WeaponKind::Cross => {
                    // Lv1〜3: 上下左右 4 方向、Lv4 以上: 斜め 4 方向も追加
                    let dirs_4: [(f32, f32); 4] = [
                        (0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0),
                    ];
                    let diag = std::f32::consts::FRAC_1_SQRT_2;
                    let dirs_8: [(f32, f32); 8] = [
                        (0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0),
                        (diag, -diag), (-diag, -diag), (diag, diag), (-diag, diag),
                    ];
                    let dirs: &[(f32, f32)] = if bcount >= 8 { &dirs_8 } else { &dirs_4 };
                    for &(dx_dir, dy_dir) in dirs {
                        w.bullets.spawn(px, py, dx_dir * BULLET_SPEED, dy_dir * BULLET_SPEED, dmg, BULLET_LIFETIME);
                    }
                    w.weapon_slots[si].cooldown_timer = cd;
                }
            }
        }
    }

    // ── パーティクル更新: 移動 + 重力 + フェードアウト ───────────
    {
        let plen = w.particles.len();
        for i in 0..plen {
            if !w.particles.alive[i] { continue; }
            w.particles.positions_x[i] += w.particles.velocities_x[i] * dt;
            w.particles.positions_y[i] += w.particles.velocities_y[i] * dt;
            w.particles.velocities_y[i] += 200.0 * dt;
            w.particles.lifetime[i] -= dt;
            if w.particles.lifetime[i] <= 0.0 {
                w.particles.kill(i);
            }
        }
    }

    // 2. 弾丸を移動・寿命更新
    let bullet_len = w.bullets.len();
    for i in 0..bullet_len {
        if !w.bullets.alive[i] {
            continue;
        }
        w.bullets.positions_x[i] += w.bullets.velocities_x[i] * dt;
        w.bullets.positions_y[i] += w.bullets.velocities_y[i] * dt;
        w.bullets.lifetime[i]    -= dt;
        if w.bullets.lifetime[i] <= 0.0 {
            w.bullets.kill(i);
            continue;
        }
        // 画面外に出た弾丸も消す
        let bx = w.bullets.positions_x[i];
        let by = w.bullets.positions_y[i];
        if bx < -100.0 || bx > SCREEN_WIDTH + 100.0 || by < -100.0 || by > SCREEN_HEIGHT + 100.0 {
            w.bullets.kill(i);
        }
    }

    // 3. 弾丸 vs 敵 衝突判定
    let hit_r = BULLET_RADIUS + ENEMY_RADIUS;
    for bi in 0..bullet_len {
        if !w.bullets.alive[bi] {
            continue;
        }
        let bx  = w.bullets.positions_x[bi];
        let by  = w.bullets.positions_y[bi];
        let dmg = w.bullets.damage[bi];

        let nearby = w.collision.dynamic.query_nearby(bx, by, hit_r);
        for ei in nearby {
            if !w.enemies.alive[ei] {
                continue;
            }
            let ex  = w.enemies.positions_x[ei] + ENEMY_RADIUS;
            let ey  = w.enemies.positions_y[ei] + ENEMY_RADIUS;
            let ddx = bx - ex;
            let ddy = by - ey;
            if ddx * ddx + ddy * ddy < hit_r * hit_r {
                w.enemies.hp[ei] -= dmg as f32;
                if w.enemies.hp[ei] <= 0.0 {
                    w.enemies.kill(ei);
                    // ── Step 13: 敵撃破でスコア加算 ──────────────
                    w.score += 10;
                    // ── Step 14: 経験値加算・レベルアップ判定 ────
                    w.exp += 5;
                    if !w.level_up_pending {
                        let required = exp_required_for_next(w.level);
                        if w.exp >= required {
                            w.level_up_pending = true;
                        }
                    }
                    // ── Step 16: 敵撃破時オレンジパーティクル ────
                    w.particles.emit(ex, ey, 8, [1.0, 0.5, 0.1, 1.0]);
                } else {
                    // ── Step 16: ヒット時黄色パーティクル ─────────
                    w.particles.emit(ex, ey, 3, [1.0, 0.9, 0.3, 1.0]);
                }
                w.bullets.kill(bi);
                break;
            }
        }
    }

    // ── Step 12: フレーム時間計測 ────────────────────────────────
    let elapsed_ms = t_start.elapsed().as_secs_f64() * 1000.0;
    w.last_frame_time_ms = elapsed_ms;
    if elapsed_ms > FRAME_BUDGET_MS {
        eprintln!(
            "[PERF] Frame budget exceeded: {:.2}ms (enemies: {})",
            elapsed_ms,
            w.enemies.count
        );
    }

    w.frame_id
}

/// プレイヤー座標を返す（Step 8）
#[rustler::nif]
fn get_player_pos(world: ResourceArc<GameWorld>) -> (f64, f64) {
    let w = world.0.lock().unwrap();
    (w.player.x as f64, w.player.y as f64)
}

/// プレイヤー HP を返す（Step 10）
#[rustler::nif]
fn get_player_hp(world: ResourceArc<GameWorld>) -> f64 {
    let w = world.0.lock().unwrap();
    w.player.hp as f64
}

/// 描画データを返す: [{x, y, kind}] のリスト
/// kind: 0 = player, 1 = enemy, 2 = bullet
#[rustler::nif]
fn get_render_data(world: ResourceArc<GameWorld>) -> Vec<(f32, f32, u8)> {
    let w = world.0.lock().unwrap();
    let mut result = Vec::with_capacity(1 + w.enemies.len() + w.bullets.len());
    result.push((w.player.x, w.player.y, 0u8));
    for i in 0..w.enemies.len() {
        if w.enemies.alive[i] {
            result.push((w.enemies.positions_x[i], w.enemies.positions_y[i], 1u8));
        }
    }
    for i in 0..w.bullets.len() {
        if w.bullets.alive[i] {
            result.push((w.bullets.positions_x[i], w.bullets.positions_y[i], 2u8));
        }
    }
    result
}

/// パーティクル描画データを返す: [(x, y, r, g, b, alpha, size)]
#[rustler::nif]
fn get_particle_data(world: ResourceArc<GameWorld>) -> Vec<(f32, f32, f32, f32, f32, f32, f32)> {
    let w = world.0.lock().unwrap();
    let mut result = Vec::with_capacity(w.particles.count);
    for i in 0..w.particles.len() {
        if !w.particles.alive[i] { continue; }
        let alpha = (w.particles.lifetime[i] / w.particles.max_lifetime[i]).clamp(0.0, 1.0);
        let c = w.particles.color[i];
        result.push((
            w.particles.positions_x[i],
            w.particles.positions_y[i],
            c[0], c[1], c[2],
            alpha,
            w.particles.size[i],
        ));
    }
    result
}

/// 現在飛んでいる弾丸数を返す（Step 11）
#[rustler::nif]
fn get_bullet_count(world: ResourceArc<GameWorld>) -> usize {
    let w = world.0.lock().unwrap();
    w.bullets.count
}

/// 直近フレームの物理ステップ処理時間をミリ秒で返す（Step 12）
#[rustler::nif]
fn get_frame_time_ms(world: ResourceArc<GameWorld>) -> f64 {
    let w = world.0.lock().unwrap();
    w.last_frame_time_ms
}

/// 現在生存している敵の数を返す（Step 12）
#[rustler::nif]
fn get_enemy_count(world: ResourceArc<GameWorld>) -> usize {
    let w = world.0.lock().unwrap();
    w.enemies.count
}

/// HUD データを一括取得（Step 13）
/// 戻り値: (hp, max_hp, score, elapsed_seconds)
#[rustler::nif]
fn get_hud_data(world: ResourceArc<GameWorld>) -> (f64, f64, u32, f64) {
    let w = world.0.lock().unwrap();
    (
        w.player.hp        as f64,
        w.player_max_hp    as f64,
        w.score,
        w.elapsed_seconds  as f64,
    )
}

// ─── Step 14: レベルアップ・武器選択 ──────────────────────────

/// 次のレベルに必要な累積経験値を返す
/// 現在の `level` から次のレベルに上がるために必要な累積 EXP を返す。
/// EXP_TABLE[level] = Lv.level → Lv.(level+1) に必要な累積 EXP。
/// 経験値は累積で管理するため、レベルアップ後も exp はリセットしない。
fn exp_required_for_next(level: u32) -> u32 {
    const EXP_TABLE: [u32; 10] = [0, 10, 25, 45, 70, 100, 135, 175, 220, 270];
    let idx = level as usize;
    if idx < EXP_TABLE.len() {
        EXP_TABLE[idx]
    } else {
        270 + (idx as u32 - 9) * 50
    }
}

/// レベルアップ関連データを一括取得（Step 14）
/// 戻り値: (exp, level, level_up_pending, exp_to_next)
#[rustler::nif]
fn get_level_up_data(world: ResourceArc<GameWorld>) -> (u32, u32, bool, u32) {
    let w = world.0.lock().unwrap();
    let exp_to_next = exp_required_for_next(w.level).saturating_sub(w.exp);
    (w.exp, w.level, w.level_up_pending, exp_to_next)
}

/// 装備中の武器スロット情報を返す（Step 17）
/// 戻り値: [(weapon_name, level)] のリスト
#[rustler::nif]
fn get_weapon_levels(world: ResourceArc<GameWorld>) -> Vec<(String, u32)> {
    let w = world.0.lock().unwrap();
    w.weapon_slots.iter().map(|s| {
        let name = match s.kind {
            WeaponKind::MagicWand => "magic_wand".to_string(),
            WeaponKind::Axe       => "axe".to_string(),
            WeaponKind::Cross     => "cross".to_string(),
        };
        (name, s.level)
    }).collect()
}

/// 武器を追加またはレベルアップし、レベルアップ待機を解除する（Step 17）
/// weapon_name: "magic_wand" | "axe" | "cross"
/// 同じ武器を選んだ場合はレベルアップ（最大 Lv.8）
/// 新規武器は最大 6 スロットまで追加可能
#[rustler::nif]
fn add_weapon(world: ResourceArc<GameWorld>, weapon_name: &str) -> Atom {
    let mut w = world.0.lock().unwrap();

    let kind = match weapon_name {
        "magic_wand" => WeaponKind::MagicWand,
        "axe"        => WeaponKind::Axe,
        "cross"      => WeaponKind::Cross,
        _            => WeaponKind::MagicWand,
    };

    const MAX_SLOTS: usize = 6;
    // 同じ武器を選んだ場合はレベルアップ
    if let Some(slot) = w.weapon_slots.iter_mut().find(|s| s.kind == kind) {
        slot.level = (slot.level + 1).min(8);
    } else if w.weapon_slots.len() < MAX_SLOTS {
        w.weapon_slots.push(WeaponSlot::new(kind));
    }
    // MAX_SLOTS かつ新規武器の場合は何もしない（選択肢に出ないはず）

    // レベルアップ処理: レベルを上げ、フラグを解除
    // exp は累積値で管理するためリセットしない
    w.level += 1;
    w.level_up_pending = false;

    ok()
}

// ─── ローダー ─────────────────────────────────────────────────

#[allow(non_local_definitions)]
fn load(env: rustler::Env, _: rustler::Term) -> bool {
    let _ = rustler::resource!(GameWorld, env);
    true
}

rustler::init!("Elixir.Game.NifBridge", load = load);

