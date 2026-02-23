mod constants;
mod item;
mod physics;
mod weapon;

use constants::{
    BULLET_LIFETIME, BULLET_RADIUS, BULLET_SPEED,
    CELL_SIZE, ENEMY_SEPARATION_FORCE,
    WEAPON_SEARCH_RADIUS,
    ENEMY_SEPARATION_RADIUS, FRAME_BUDGET_MS,
    INVINCIBLE_DURATION, PLAYER_RADIUS, PLAYER_SIZE, PLAYER_SPEED,
    SCREEN_HEIGHT, SCREEN_WIDTH,
};
use item::{ItemKind, ItemWorld};
use weapon::{WeaponKind, WeaponSlot, MAX_WEAPON_SLOTS};
use physics::rng::SimpleRng;
use physics::separation::{apply_separation, EnemySeparation};
use physics::spatial_hash::CollisionWorld;
use rayon::prelude::*;
use rustler::{Atom, NifResult, ResourceArc};
use std::sync::RwLock;

rustler::atoms! {
    ok,
    slime,
    bat,
    golem,
    // 武器種別アトム
    magic_wand,
    axe,
    cross,
    whip,
    fireball,
    lightning,
    // level_up 通知アトム
    level_up,
    no_change,
    // Step 24: ボス種別アトム
    slime_king,
    bat_lord,
    stone_golem,
    // ゲーム状態アトム
    alive,
    dead,
    none,
    // Step 26: イベントバス用アトム
    enemy_killed,
    player_damaged,
    level_up_event,
    item_pickup,
    boss_defeated,
}

/// Step 26: フレーム内で発生したゲームイベント（EventBus 用）
#[derive(Debug, Clone)]
pub enum FrameEvent {
    EnemyKilled  { enemy_kind: u8, weapon_kind: u8 },
    PlayerDamaged { damage: f32 },
    LevelUp      { new_level: u32 },
    ItemPickup   { item_kind: u8 },
    BossDefeated { boss_kind: u8 },
}

// ─── 敵タイプ ─────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum EnemyKind {
    Slime = 0,
    Bat   = 1,
    Golem = 2,
}

impl EnemyKind {
    pub fn max_hp(&self) -> f32 {
        match self { Self::Slime => 30.0, Self::Bat => 15.0, Self::Golem => 150.0 }
    }
    pub fn speed(&self) -> f32 {
        match self { Self::Slime => 80.0, Self::Bat => 160.0, Self::Golem => 40.0 }
    }
    pub fn radius(&self) -> f32 {
        match self { Self::Slime => 20.0, Self::Bat => 12.0, Self::Golem => 32.0 }
    }
    pub fn exp_reward(&self) -> u32 {
        match self { Self::Slime => 5, Self::Bat => 3, Self::Golem => 20 }
    }
    pub fn damage_per_sec(&self) -> f32 {
        match self { Self::Slime => 20.0, Self::Bat => 10.0, Self::Golem => 40.0 }
    }
    /// レンダラーに渡す kind 値（0=player, 1=slime, 2=bat, 3=golem）
    pub fn render_kind(&self) -> u8 {
        match self { Self::Slime => 1, Self::Bat => 2, Self::Golem => 3 }
    }
    pub fn from_atom(atom: Atom) -> Self {
        // rustler::atoms! で定義したアトム関数と直接比較する
        // bat() / golem() は初回呼び出し時に BEAM アトムテーブルに登録される
        if atom == bat() {
            Self::Bat
        } else if atom == golem() {
            Self::Golem
        } else {
            Self::Slime
        }
    }
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
#[derive(Clone)]
pub struct EnemyWorld {
    pub positions_x:  Vec<f32>,
    pub positions_y:  Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub speeds:       Vec<f32>,
    pub hp:           Vec<f32>,
    pub alive:        Vec<bool>,
    pub kinds:        Vec<EnemyKind>,
    pub count:        usize,
    /// 分離パス用の作業バッファ（毎フレーム再利用してアロケーションを回避）
    pub sep_x:        Vec<f32>,
    pub sep_y:        Vec<f32>,
    /// 近隣クエリ結果の再利用バッファ（毎フレームのヒープアロケーションを回避）
    pub neighbor_buf: Vec<usize>,
    /// 空きスロットのインデックススタック — O(1) でスロットを取得・返却
    free_list:        Vec<usize>,
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
            kinds:        Vec::new(),
            count:        0,
            sep_x:        Vec::new(),
            sep_y:        Vec::new(),
            neighbor_buf: Vec::new(),
            free_list:    Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.positions_x.len()
    }

    pub fn kill(&mut self, i: usize) {
        if self.alive[i] {
            self.alive[i] = false;
            self.count = self.count.saturating_sub(1);
            self.free_list.push(i);
        }
    }

    /// 指定タイプの敵を `positions` の座標にスポーン（O(1) でスロット取得）
    pub fn spawn(&mut self, positions: &[(f32, f32)], kind: EnemyKind) {
        let speed  = kind.speed();
        let max_hp = kind.max_hp();

        for &(x, y) in positions {
            if let Some(i) = self.free_list.pop() {
                // O(1): フリーリストから再利用
                self.positions_x[i]  = x;
                self.positions_y[i]  = y;
                self.velocities_x[i] = 0.0;
                self.velocities_y[i] = 0.0;
                self.speeds[i]       = speed;
                self.hp[i]           = max_hp;
                self.alive[i]        = true;
                self.kinds[i]        = kind;
                self.sep_x[i]        = 0.0;
                self.sep_y[i]        = 0.0;
            } else {
                self.positions_x.push(x);
                self.positions_y.push(y);
                self.velocities_x.push(0.0);
                self.velocities_y.push(0.0);
                self.speeds.push(speed);
                self.hp.push(max_hp);
                self.alive.push(true);
                self.kinds.push(kind);
                self.sep_x.push(0.0);
                self.sep_y.push(0.0);
            }
            self.count += 1;
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

/// 弾丸の描画種別（renderer に渡す kind 値）
pub const BULLET_KIND_NORMAL:    u8 = 4;  // MagicWand / Axe / Cross（黄色い円）
pub const BULLET_KIND_FIREBALL:  u8 = 8;  // Fireball（赤橙の炎球）
pub const BULLET_KIND_LIGHTNING: u8 = 9;  // Lightning（水色の電撃球）
pub const BULLET_KIND_WHIP:      u8 = 10; // Whip（黄緑の弧状）
// 11=SlimeKing, 12=BatLord, 13=StoneGolem（ボス render_kind と共有）
pub const BULLET_KIND_ROCK:      u8 = 14; // StoneGolem の岩弾

// ─── 弾丸 SoA ─────────────────────────────────────────────────
pub struct BulletWorld {
    pub positions_x:  Vec<f32>,
    pub positions_y:  Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub damage:       Vec<i32>,
    pub lifetime:     Vec<f32>,
    pub alive:        Vec<bool>,
    /// true の弾丸は敵に当たっても消えずに貫通する（Fireball 用）
    pub piercing:     Vec<bool>,
    /// 描画種別（BULLET_KIND_* 定数）
    pub render_kind:  Vec<u8>,
    /// Step 26: 発射元武器（EnemyKilled イベント用、WeaponKind::as_u8()）
    pub weapon_kind:  Vec<u8>,
    pub count:        usize,
    /// 空きスロットのインデックススタック — O(1) でスロットを取得・返却
    free_list:        Vec<usize>,
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
            piercing:     Vec::new(),
            render_kind:  Vec::new(),
            weapon_kind:  Vec::new(),
            count:        0,
            free_list:    Vec::new(),
        }
    }

    pub fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32, damage: i32, lifetime: f32, weapon_kind: u8) {
        self.spawn_ex(x, y, vx, vy, damage, lifetime, false, BULLET_KIND_NORMAL, weapon_kind);
    }

    pub fn spawn_piercing(&mut self, x: f32, y: f32, vx: f32, vy: f32, damage: i32, lifetime: f32, weapon_kind: u8) {
        self.spawn_ex(x, y, vx, vy, damage, lifetime, true, BULLET_KIND_FIREBALL, weapon_kind);
    }

    /// ダメージ 0・短命の表示専用エフェクト弾を生成する（Whip / Lightning 用）
    pub fn spawn_effect(&mut self, x: f32, y: f32, lifetime: f32, render_kind: u8) {
        self.spawn_ex(x, y, 0.0, 0.0, 0, lifetime, false, render_kind, 0);
    }

    fn spawn_ex(&mut self, x: f32, y: f32, vx: f32, vy: f32, damage: i32, lifetime: f32, piercing: bool, render_kind: u8, weapon_kind: u8) {
        if let Some(i) = self.free_list.pop() {
            // O(1): フリーリストから空きスロットを取得
            self.positions_x[i]  = x;
            self.positions_y[i]  = y;
            self.velocities_x[i] = vx;
            self.velocities_y[i] = vy;
            self.damage[i]       = damage;
            self.lifetime[i]     = lifetime;
            self.alive[i]        = true;
            self.piercing[i]     = piercing;
            self.render_kind[i]  = render_kind;
            self.weapon_kind[i]  = weapon_kind;
        } else {
            // フリーリストが空なら末尾に追加
            self.positions_x.push(x);
            self.positions_y.push(y);
            self.velocities_x.push(vx);
            self.velocities_y.push(vy);
            self.damage.push(damage);
            self.lifetime.push(lifetime);
            self.alive.push(true);
            self.piercing.push(piercing);
            self.render_kind.push(render_kind);
            self.weapon_kind.push(weapon_kind);
        }
        self.count += 1;
    }

    pub fn kill(&mut self, i: usize) {
        if self.alive[i] {
            self.alive[i] = false;
            self.count = self.count.saturating_sub(1);
            self.free_list.push(i);
        }
    }

    pub fn len(&self) -> usize {
        self.positions_x.len()
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
    /// 空きスロットのインデックススタック — O(1) でスロットを取得・返却
    free_list:        Vec<usize>,
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
            free_list:    Vec::new(),
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
        if let Some(i) = self.free_list.pop() {
            // O(1): フリーリストから空きスロットを取得
            self.positions_x[i]  = x;
            self.positions_y[i]  = y;
            self.velocities_x[i] = vx;
            self.velocities_y[i] = vy;
            self.lifetime[i]     = lifetime;
            self.max_lifetime[i] = lifetime;
            self.color[i]        = color;
            self.size[i]         = size;
            self.alive[i]        = true;
        } else {
            // フリーリストが空なら末尾に追加
            self.positions_x.push(x);
            self.positions_y.push(y);
            self.velocities_x.push(vx);
            self.velocities_y.push(vy);
            self.lifetime.push(lifetime);
            self.max_lifetime.push(lifetime);
            self.color.push(color);
            self.size.push(size);
            self.alive.push(true);
        }
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
            self.free_list.push(i);
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

/// 指定インデックスを除外した最近接の生存敵インデックスを返す（Lightning チェーン用）
pub fn find_nearest_enemy_excluding(
    enemies: &EnemyWorld,
    px: f32,
    py: f32,
    exclude: &[usize],
) -> Option<usize> {
    let mut min_dist = f32::MAX;
    let mut nearest  = None;
    for i in 0..enemies.len() {
        if !enemies.alive[i] || exclude.contains(&i) {
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

/// 二乗距離（sqrt を避けて高速化）
#[inline]
fn dist_sq(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x1 - x2;
    let dy = y1 - y2;
    dx * dx + dy * dy
}

/// Spatial Hash を使った高速最近接探索
/// search_radius 内に候補がいなければ全探索にフォールバック
pub fn find_nearest_enemy_spatial(
    collision: &CollisionWorld,
    enemies: &EnemyWorld,
    px: f32,
    py: f32,
    search_radius: f32,
) -> Option<usize> {
    let candidates = collision.dynamic.query_nearby(px, py, search_radius);

    let result = candidates
        .iter()
        .filter(|&&i| i < enemies.len() && enemies.alive[i])
        .map(|&i| (i, dist_sq(enemies.positions_x[i], enemies.positions_y[i], px, py)))
        .min_by(|(_, da), (_, db)| da.partial_cmp(db).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i);

    // 半径内に誰もいなければ全探索（フォールバック）
    result.or_else(|| find_nearest_enemy(enemies, px, py))
}

/// Spatial Hash を使った高速最近接探索（除外リスト付き・Lightning チェーン用）
/// search_radius 内の候補から exclude を除外して最近接を返す
pub fn find_nearest_enemy_spatial_excluding(
    collision: &CollisionWorld,
    enemies: &EnemyWorld,
    px: f32,
    py: f32,
    search_radius: f32,
    exclude: &[usize],
) -> Option<usize> {
    let candidates = collision.dynamic.query_nearby(px, py, search_radius);

    let result = candidates
        .iter()
        .filter(|&&i| {
            i < enemies.len()
                && enemies.alive[i]
                && !exclude.contains(&i)
        })
        .map(|&i| (i, dist_sq(enemies.positions_x[i], enemies.positions_y[i], px, py)))
        .min_by(|(_, da), (_, db)| da.partial_cmp(db).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i);

    // 半径内に誰もいなければ全探索（フォールバック）
    result.or_else(|| find_nearest_enemy_excluding(enemies, px, py, exclude))
}

/// 1 体分の Chase AI（スカラー版・SIMD フォールバック用）
#[inline]
fn scalar_chase_one(
    enemies: &mut EnemyWorld,
    i: usize,
    player_x: f32,
    player_y: f32,
    dt: f32,
) {
    let dx = player_x - enemies.positions_x[i];
    let dy = player_y - enemies.positions_y[i];
    let dist = (dx * dx + dy * dy).sqrt().max(0.001);
    let speed = enemies.speeds[i];
    enemies.velocities_x[i] = (dx / dist) * speed;
    enemies.velocities_y[i] = (dy / dist) * speed;
    enemies.positions_x[i] += enemies.velocities_x[i] * dt;
    enemies.positions_y[i] += enemies.velocities_y[i] * dt;
}

/// SIMD（SSE2）版 Chase AI — x86_64 専用
/// rayon 版と同じ結果を返すが、4 要素を同時処理する
#[cfg(target_arch = "x86_64")]
pub fn update_chase_ai_simd(
    enemies: &mut EnemyWorld,
    player_x: f32,
    player_y: f32,
    dt: f32,
) {
    use std::arch::x86_64::*;

    let len = enemies.len();
    let simd_len = (len / 4) * 4;

    unsafe {
        let px4 = _mm_set1_ps(player_x);
        let py4 = _mm_set1_ps(player_y);
        let dt4 = _mm_set1_ps(dt);
        let eps4 = _mm_set1_ps(0.001_f32);

        for base in (0..simd_len).step_by(4) {
            // 4 要素を同時ロード
            let ex = _mm_loadu_ps(enemies.positions_x[base..].as_ptr());
            let ey = _mm_loadu_ps(enemies.positions_y[base..].as_ptr());
            let sp = _mm_loadu_ps(enemies.speeds[base..].as_ptr());

            // 方向ベクトルを計算
            let dx = _mm_sub_ps(px4, ex);
            let dy = _mm_sub_ps(py4, ey);

            // 距離の二乗
            let dist_sq = _mm_add_ps(_mm_mul_ps(dx, dx), _mm_mul_ps(dy, dy));

            // 逆平方根（高速近似）— max(eps) でゼロ除算を防ぐ
            let dist_sq_safe = _mm_max_ps(dist_sq, eps4);
            let inv_dist = _mm_rsqrt_ps(dist_sq_safe);

            // 速度を計算
            let vx = _mm_mul_ps(_mm_mul_ps(dx, inv_dist), sp);
            let vy = _mm_mul_ps(_mm_mul_ps(dy, inv_dist), sp);

            // 位置を更新
            let new_ex = _mm_add_ps(ex, _mm_mul_ps(vx, dt4));
            let new_ey = _mm_add_ps(ey, _mm_mul_ps(vy, dt4));

            // alive フラグからマスクを作成（分岐を排除してブレンディングで生存者のみ更新）
            let alive_mask = _mm_castsi128_ps(_mm_set_epi32(
                if enemies.alive[base + 3] { -1i32 } else { 0 },
                if enemies.alive[base + 2] { -1i32 } else { 0 },
                if enemies.alive[base + 1] { -1i32 } else { 0 },
                if enemies.alive[base + 0] { -1i32 } else { 0 },
            ));

            // SSE2 のビット演算でブレンディング（alive のとき新値、dead のとき旧値）
            let old_vx = _mm_loadu_ps(enemies.velocities_x[base..].as_ptr());
            let old_vy = _mm_loadu_ps(enemies.velocities_y[base..].as_ptr());

            let final_ex = _mm_or_ps(
                _mm_andnot_ps(alive_mask, ex),
                _mm_and_ps(alive_mask, new_ex),
            );
            let final_ey = _mm_or_ps(
                _mm_andnot_ps(alive_mask, ey),
                _mm_and_ps(alive_mask, new_ey),
            );
            let final_vx = _mm_or_ps(
                _mm_andnot_ps(alive_mask, old_vx),
                _mm_and_ps(alive_mask, vx),
            );
            let final_vy = _mm_or_ps(
                _mm_andnot_ps(alive_mask, old_vy),
                _mm_and_ps(alive_mask, vy),
            );

            // 書き戻し
            _mm_storeu_ps(enemies.positions_x[base..].as_mut_ptr(), final_ex);
            _mm_storeu_ps(enemies.positions_y[base..].as_mut_ptr(), final_ey);
            _mm_storeu_ps(enemies.velocities_x[base..].as_mut_ptr(), final_vx);
            _mm_storeu_ps(enemies.velocities_y[base..].as_mut_ptr(), final_vy);
        }

        // 残り要素をスカラーで処理
        for i in simd_len..len {
            if enemies.alive[i] {
                scalar_chase_one(enemies, i, player_x, player_y, dt);
            }
        }
    }
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


// ─── Step 24: ボス種別 ────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum BossKind {
    SlimeKing,
    BatLord,
    StoneGolem,
}

impl BossKind {
    pub fn max_hp(&self) -> f32 {
        match self { Self::SlimeKing => 1000.0, Self::BatLord => 2000.0, Self::StoneGolem => 5000.0 }
    }
    pub fn speed(&self) -> f32 {
        match self { Self::SlimeKing => 60.0, Self::BatLord => 200.0, Self::StoneGolem => 30.0 }
    }
    pub fn radius(&self) -> f32 {
        match self { Self::SlimeKing => 48.0, Self::BatLord => 48.0, Self::StoneGolem => 64.0 }
    }
    pub fn exp_reward(&self) -> u32 {
        match self { Self::SlimeKing => 200, Self::BatLord => 400, Self::StoneGolem => 800 }
    }
    pub fn damage_per_sec(&self) -> f32 {
        match self { Self::SlimeKing => 30.0, Self::BatLord => 50.0, Self::StoneGolem => 80.0 }
    }
    pub fn special_interval(&self) -> f32 {
        match self { Self::SlimeKing => 5.0, Self::BatLord => 4.0, Self::StoneGolem => 6.0 }
    }
    pub fn render_kind(&self) -> u8 {
        match self { Self::SlimeKing => 11, Self::BatLord => 12, Self::StoneGolem => 13 }
    }
    pub fn from_atom(atom: Atom) -> Option<Self> {
        if atom == slime_king() { Some(Self::SlimeKing) }
        else if atom == bat_lord() { Some(Self::BatLord) }
        else if atom == stone_golem() { Some(Self::StoneGolem) }
        else { None }
    }
}

pub struct BossState {
    pub kind:             BossKind,
    pub x:                f32,
    pub y:                f32,
    pub hp:               f32,
    pub max_hp:           f32,
    pub phase_timer:      f32,
    pub invincible:       bool,
    pub invincible_timer: f32,
    pub is_dashing:       bool,
    pub dash_timer:       f32,
    pub dash_vx:          f32,
    pub dash_vy:          f32,
}

impl BossState {
    pub fn new(kind: BossKind, x: f32, y: f32) -> Self {
        let max_hp = kind.max_hp();
        Self {
            kind,
            x, y,
            hp: max_hp,
            max_hp,
            phase_timer: kind.special_interval(),
            invincible: false,
            invincible_timer: 0.0,
            is_dashing: false,
            dash_timer: 0.0,
            dash_vx: 0.0,
            dash_vy: 0.0,
        }
    }
}

// ─── ゲームワールド ───────────────────────────────────────────
pub struct GameWorldInner {
    pub frame_id:           u32,
    pub player:             PlayerState,
    pub enemies:            EnemyWorld,
    pub bullets:            BulletWorld,
    pub particles:          ParticleWorld,
    /// ─── Step 19: アイテム ────────────────────────────────────
    pub items:              ItemWorld,
    /// 磁石エフェクト残り時間（秒）
    pub magnet_timer:       f32,
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
    /// ─── Step 24: ボスエネミー ────────────────────────────────
    pub boss:               Option<BossState>,
    /// Step 26: このフレームで発生したイベント（毎フレーム drain される）
    pub frame_events:       Vec<FrameEvent>,
}

impl GameWorldInner {
    /// レベルアップ処理を完了する（武器選択・スキップ共通）
    fn complete_level_up(&mut self) {
        self.level += 1;
        self.level_up_pending = false;
    }

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

pub struct GameWorld(pub RwLock<GameWorldInner>);

// ─── NIF 関数 ─────────────────────────────────────────────────

#[rustler::nif]
fn add(a: i64, b: i64) -> NifResult<i64> {
    Ok(a + b)
}

#[rustler::nif]
fn create_world() -> ResourceArc<GameWorld> {
    ResourceArc::new(GameWorld(RwLock::new(GameWorldInner {
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
        items:              ItemWorld::new(),
        magnet_timer:       0.0,
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
        boss:               None,
        frame_events:       Vec::new(),
    })))
}

/// RwLock の PoisonError を NifResult に変換するヘルパー
#[inline]
fn lock_poisoned_err() -> rustler::Error {
    rustler::Error::RaiseAtom("lock_poisoned")
}

/// プレイヤーの入力方向を設定（Step 8）
#[rustler::nif]
fn set_player_input(world: ResourceArc<GameWorld>, dx: f64, dy: f64) -> NifResult<Atom> {
    let mut w = world.0.write().map_err(|_| lock_poisoned_err())?;
    w.player.input_dx = dx as f32;
    w.player.input_dy = dy as f32;
    Ok(ok())
}

/// 敵をスポーン（Step 9 / Step 18）
/// kind: :slime | :bat | :golem
#[rustler::nif]
fn spawn_enemies(world: ResourceArc<GameWorld>, kind: Atom, count: usize) -> NifResult<Atom> {
    let mut w = world.0.write().map_err(|_| lock_poisoned_err())?;
    let enemy_kind = EnemyKind::from_atom(kind);
    // rng の借用を先に終わらせてから enemies に渡す
    let positions: Vec<(f32, f32)> = (0..count)
        .map(|_| spawn_position_outside(&mut w.rng, SCREEN_WIDTH, SCREEN_HEIGHT))
        .collect();
    w.enemies.spawn(&positions, enemy_kind);
    Ok(ok())
}

/// 物理ステップ: プレイヤー移動 + Chase AI + 衝突判定（Step 8/9/10/12）
#[rustler::nif(schedule = "DirtyCpu")]
fn physics_step(world: ResourceArc<GameWorld>, delta_ms: f64) -> NifResult<u32> {
    let t_start = std::time::Instant::now();

    let mut w = world.0.write().map_err(|_| lock_poisoned_err())?;
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

    // Chase AI（x86_64 では SIMD 版、それ以外は rayon 版）
    let px = w.player.x + PLAYER_RADIUS;
    let py = w.player.y + PLAYER_RADIUS;
    #[cfg(target_arch = "x86_64")]
    update_chase_ai_simd(&mut w.enemies, px, py, dt);
    #[cfg(not(target_arch = "x86_64"))]
    update_chase_ai(&mut w.enemies, px, py, dt);

    // 敵同士の重なりを解消する分離パス
    apply_separation(&mut w.enemies, ENEMY_SEPARATION_RADIUS, ENEMY_SEPARATION_FORCE, dt);

    // ── Step 10: 衝突判定（Spatial Hash）────────────────────────
    // 1. 動的 Spatial Hash を再構築
    w.rebuild_collision();

    // 無敵タイマーを更新
    if w.player.invincible_timer > 0.0 {
        w.player.invincible_timer = (w.player.invincible_timer - dt).max(0.0);
    }

    // 2. プレイヤー周辺の敵を取得して円-円判定
    // 最大の敵半径（Golem: 32px）を考慮してクエリ半径を広げる
    let max_enemy_radius = 32.0_f32;
    let query_radius = PLAYER_RADIUS + max_enemy_radius;
    let candidates = w.collision.dynamic.query_nearby(px, py, query_radius);

    for idx in candidates {
        if !w.enemies.alive[idx] {
            continue;
        }
        let kind = w.enemies.kinds[idx];
        let enemy_r = kind.radius();
        let hit_radius = PLAYER_RADIUS + enemy_r;
        let ex = w.enemies.positions_x[idx] + enemy_r;
        let ey = w.enemies.positions_y[idx] + enemy_r;
        let ddx = px - ex;
        let ddy = py - ey;
        let dist_sq = ddx * ddx + ddy * ddy;

        if dist_sq < hit_radius * hit_radius {
            // 敵→プレイヤーへのダメージ（無敵時間中は無効）
            if w.player.invincible_timer <= 0.0 && w.player.hp > 0.0 {
                let dmg = kind.damage_per_sec() * dt;
                w.player.hp = (w.player.hp - dmg).max(0.0);
                w.player.invincible_timer = INVINCIBLE_DURATION;
                w.frame_events.push(FrameEvent::PlayerDamaged { damage: dmg });
                // 赤いパーティクルをプレイヤー位置に発生
                let ppx = w.player.x + PLAYER_RADIUS;
                let ppy = w.player.y + PLAYER_RADIUS;
                w.particles.emit(ppx, ppy, 6, [1.0, 0.15, 0.15, 1.0]);
            }
        }
    }

            // ── Step 11/14/17/21: 武器スロット発射処理 ──────────────────
    // level_up_pending 中は発射を止めてゲームを一時停止する
    if !w.level_up_pending {
        // プレイヤーの移動方向（Whip の向き計算用）
        let facing_angle = {
            let fdx = w.player.input_dx;
            let fdy = w.player.input_dy;
            if fdx * fdx + fdy * fdy > 0.0001 {
                fdy.atan2(fdx)
            } else {
                // 停止中は右向きをデフォルトとする
                0.0_f32
            }
        };

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
            let level = w.weapon_slots[si].level;
            let bcount = w.weapon_slots[si].bullet_count();
            match kind {
                WeaponKind::MagicWand => {
                    if let Some(ti) = find_nearest_enemy_spatial(&w.collision, &w.enemies, px, py, WEAPON_SEARCH_RADIUS) {
                        let target_r = w.enemies.kinds[ti].radius();
                        let tx   = w.enemies.positions_x[ti] + target_r;
                        let ty   = w.enemies.positions_y[ti] + target_r;
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
                            w.bullets.spawn(px, py, vx, vy, dmg, BULLET_LIFETIME, kind.as_u8());
                        }
                        w.weapon_slots[si].cooldown_timer = cd;
                    }
                }
                WeaponKind::Axe => {
                    // 上方向に直進（簡易実装）
                    w.bullets.spawn(px, py, 0.0, -BULLET_SPEED, dmg, BULLET_LIFETIME, kind.as_u8());
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
                        w.bullets.spawn(px, py, dx_dir * BULLET_SPEED, dy_dir * BULLET_SPEED, dmg, BULLET_LIFETIME, kind.as_u8());
                    }
                    w.weapon_slots[si].cooldown_timer = cd;
                }
                // ── Step 21: Whip ──────────────────────────────────────────
                WeaponKind::Whip => {
                    // プレイヤーの移動方向に扇状の判定を出す（弾丸を生成しない直接判定）
                    let whip_range = kind.whip_range(level);
                    let whip_half_angle = std::f32::consts::PI * 0.3; // 108度 / 2 = 54度
                    // facing_angle 方向の中間点にエフェクト弾を生成（kind=10: 黄緑の横長楕円）
                    let eff_x = px + facing_angle.cos() * whip_range * 0.5;
                    let eff_y = py + facing_angle.sin() * whip_range * 0.5;
                    w.bullets.spawn_effect(eff_x, eff_y, 0.12, BULLET_KIND_WHIP);
                    // 空間ハッシュで範囲内の候補のみ取得し、全敵ループを回避
                    let whip_range_sq = whip_range * whip_range;
                    let candidates = w.collision.dynamic.query_nearby(px, py, whip_range);
                    for ei in candidates {
                        if !w.enemies.alive[ei] { continue; }
                        let ex = w.enemies.positions_x[ei];
                        let ey = w.enemies.positions_y[ei];
                        let ddx = ex - px;
                        let ddy = ey - py;
                        // sqrt を避けて二乗比較で正確な円形クリップ
                        if ddx * ddx + ddy * ddy > whip_range_sq { continue; }
                        let angle = ddy.atan2(ddx);
                        // π/-π をまたぐ場合に正しく動作するよう -π〜π に正規化
                        let mut diff = angle - facing_angle;
                        if diff >  std::f32::consts::PI { diff -= std::f32::consts::TAU; }
                        if diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
                        if diff.abs() < whip_half_angle {
                            let enemy_r = w.enemies.kinds[ei].radius();
                            let hit_x = ex + enemy_r;
                            let hit_y = ey + enemy_r;
                            w.enemies.hp[ei] -= dmg as f32;
                            if w.enemies.hp[ei] <= 0.0 {
                                let kind_e = w.enemies.kinds[ei];
                                w.enemies.kill(ei);
                                w.frame_events.push(FrameEvent::EnemyKilled {
                                    enemy_kind:  kind_e as u8,
                                    weapon_kind: kind.as_u8(),
                                });
                                w.score += kind_e.exp_reward() * 2;
                                w.exp   += kind_e.exp_reward();
                                if !w.level_up_pending {
                                    let required = exp_required_for_next(w.level);
                                    if w.exp >= required {
                                        let new_lv = w.level + 1;
                                        w.level_up_pending = true;
                                        w.frame_events.push(FrameEvent::LevelUp { new_level: new_lv });
                                    }
                                }
                                let pc = match kind_e {
                                    EnemyKind::Slime => [1.0, 0.5, 0.1, 1.0],
                                    EnemyKind::Bat   => [0.7, 0.2, 0.9, 1.0],
                                    EnemyKind::Golem => [0.6, 0.6, 0.6, 1.0],
                                };
                                w.particles.emit(hit_x, hit_y, 8, pc);
                                let roll = w.rng.next_u32() % 100;
                                let (item_kind, item_value) = if roll < 2 {
                                    (item::ItemKind::Magnet, 0)
                                } else if roll < 7 {
                                    (item::ItemKind::Potion, 20)
                                } else {
                                    (item::ItemKind::Gem, kind_e.exp_reward())
                                };
                                w.items.spawn(hit_x, hit_y, item_kind, item_value);
                            } else {
                                w.particles.emit(hit_x, hit_y, 3, [1.0, 0.6, 0.1, 1.0]);
                            }
                        }
                    }
                    // Step 24: Whip vs ボス
                    {
                        let whip_range_sq = whip_range * whip_range;
                        let boss_hit_pos: Option<(f32, f32)> = if let Some(ref boss) = w.boss {
                            if !boss.invincible {
                                let ddx = boss.x - px;
                                let ddy = boss.y - py;
                                if ddx * ddx + ddy * ddy <= whip_range_sq {
                                    let angle = ddy.atan2(ddx);
                                    let mut diff = angle - facing_angle;
                                    if diff >  std::f32::consts::PI { diff -= std::f32::consts::TAU; }
                                    if diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
                                    if diff.abs() < whip_half_angle { Some((boss.x, boss.y)) } else { None }
                                } else { None }
                            } else { None }
                        } else { None };
                        if let Some((bx, by)) = boss_hit_pos {
                            if let Some(ref mut boss) = w.boss { boss.hp -= dmg as f32; }
                            w.particles.emit(bx, by, 4, [1.0, 0.8, 0.2, 1.0]);
                        }
                    }
                    w.weapon_slots[si].cooldown_timer = cd;
                }
                // ── Step 21: Fireball ──────────────────────────────────────
                WeaponKind::Fireball => {
                    // 最近接敵に向かって貫通弾を発射
                    if let Some(ti) = find_nearest_enemy_spatial(&w.collision, &w.enemies, px, py, WEAPON_SEARCH_RADIUS) {
                        let target_r = w.enemies.kinds[ti].radius();
                        let tx  = w.enemies.positions_x[ti] + target_r;
                        let ty  = w.enemies.positions_y[ti] + target_r;
                        let bdx = tx - px;
                        let bdy = ty - py;
                        let base_angle = bdy.atan2(bdx);
                        let vx = base_angle.cos() * BULLET_SPEED;
                        let vy = base_angle.sin() * BULLET_SPEED;
                        w.bullets.spawn_piercing(px, py, vx, vy, dmg, BULLET_LIFETIME, kind.as_u8());
                        w.weapon_slots[si].cooldown_timer = cd;
                    }
                }
                // ── Step 21: Lightning ─────────────────────────────────────
                WeaponKind::Lightning => {
                    // 最近接敵から始まり、最大 chain_count 体に連鎖
                    let chain_count = kind.lightning_chain_count(level);
                    // chain_count は最大 6 程度と小さいため Vec で十分（HashSet 不要）
                    let mut hit_vec: Vec<usize> = Vec::with_capacity(chain_count);
                    // 最初はプレイヤー位置から最近接敵を探す（空間ハッシュで候補を絞る）
                    let mut current = find_nearest_enemy_spatial(&w.collision, &w.enemies, px, py, WEAPON_SEARCH_RADIUS);
                    #[allow(unused_assignments)]
                    let mut next_search_x = px;
                    #[allow(unused_assignments)]
                    let mut next_search_y = py;
                    for _ in 0..chain_count {
                        if let Some(ei) = current {
                            let enemy_r = w.enemies.kinds[ei].radius();
                            let hit_x = w.enemies.positions_x[ei] + enemy_r;
                            let hit_y = w.enemies.positions_y[ei] + enemy_r;
                            w.enemies.hp[ei] -= dmg as f32;
                            // 電撃エフェクト弾（kind=9: 水色の電撃球）+ パーティクル
                            w.bullets.spawn_effect(hit_x, hit_y, 0.10, BULLET_KIND_LIGHTNING);
                            w.particles.emit(hit_x, hit_y, 5, [0.3, 0.8, 1.0, 1.0]);
                            if w.enemies.hp[ei] <= 0.0 {
                                let kind_e = w.enemies.kinds[ei];
                                w.enemies.kill(ei);
                                w.frame_events.push(FrameEvent::EnemyKilled {
                                    enemy_kind:  kind_e as u8,
                                    weapon_kind: kind.as_u8(),
                                });
                                w.score += kind_e.exp_reward() * 2;
                                w.exp   += kind_e.exp_reward();
                                if !w.level_up_pending {
                                    let required = exp_required_for_next(w.level);
                                    if w.exp >= required {
                                        let new_lv = w.level + 1;
                                        w.level_up_pending = true;
                                        w.frame_events.push(FrameEvent::LevelUp { new_level: new_lv });
                                    }
                                }
                                let roll = w.rng.next_u32() % 100;
                                let (item_kind, item_value) = if roll < 2 {
                                    (item::ItemKind::Magnet, 0)
                                } else if roll < 7 {
                                    (item::ItemKind::Potion, 20)
                                } else {
                                    (item::ItemKind::Gem, kind_e.exp_reward())
                                };
                                w.items.spawn(hit_x, hit_y, item_kind, item_value);
                            }
                            hit_vec.push(ei);
                            next_search_x = hit_x;
                            next_search_y = hit_y;
                            current = find_nearest_enemy_spatial_excluding(
                                &w.collision, &w.enemies,
                                next_search_x, next_search_y,
                                WEAPON_SEARCH_RADIUS, &hit_vec,
                            );
                        } else {
                            break;
                        }
                    }
                    // Step 24: Lightning vs ボス（600px 以内なら連鎖先としてダメージ）
                    {
                        let boss_hit_pos: Option<(f32, f32)> = if let Some(ref boss) = w.boss {
                            if !boss.invincible {
                                let ddx = boss.x - px;
                                let ddy = boss.y - py;
                                if ddx * ddx + ddy * ddy < 600.0 * 600.0 {
                                    Some((boss.x, boss.y))
                                } else { None }
                            } else { None }
                        } else { None };
                        if let Some((bx, by)) = boss_hit_pos {
                            if let Some(ref mut boss) = w.boss { boss.hp -= dmg as f32; }
                            w.bullets.spawn_effect(bx, by, 0.10, BULLET_KIND_LIGHTNING);
                            w.particles.emit(bx, by, 5, [0.3, 0.8, 1.0, 1.0]);
                        }
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

    // ── Step 19: アイテム更新（磁石エフェクト + 自動収集） ─────
    {
        // 磁石タイマー更新
        if w.magnet_timer > 0.0 {
            w.magnet_timer = (w.magnet_timer - dt).max(0.0);
        }

        // 磁石エフェクト: アクティブ中は宝石がプレイヤーに向かって飛んでくる
        if w.magnet_timer > 0.0 {
            let item_len = w.items.len();
            for i in 0..item_len {
                if !w.items.alive[i] { continue; }
                if w.items.kinds[i] != ItemKind::Gem { continue; }
                let dx = px - w.items.positions_x[i];
                let dy = py - w.items.positions_y[i];
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                w.items.positions_x[i] += (dx / dist) * 300.0 * dt;
                w.items.positions_y[i] += (dy / dist) * 300.0 * dt;
            }
        }

        // 自動収集判定（通常: 60px、磁石中: 全画面）
        let collect_r = if w.magnet_timer > 0.0 { 9999.0_f32 } else { 60.0_f32 };
        let collect_r_sq = collect_r * collect_r;
        let item_len = w.items.len();
        for i in 0..item_len {
            if !w.items.alive[i] { continue; }
            let dx = px - w.items.positions_x[i];
            let dy = py - w.items.positions_y[i];
            if dx * dx + dy * dy <= collect_r_sq {
                let item_k = w.items.kinds[i];
                match item_k {
                    ItemKind::Gem => {
                        // EXP は既に撃破時に加算済みのため、ここでは収集のみ
                    }
                    ItemKind::Potion => {
                        // HP 回復（最大 HP を超えない）
                        w.player.hp = (w.player.hp + w.items.value[i] as f32)
                            .min(w.player_max_hp);
                        // 回復パーティクル（緑）
                        w.particles.emit(px, py, 6, [0.2, 1.0, 0.4, 1.0]);
                    }
                    ItemKind::Magnet => {
                        // 磁石エフェクトを 10 秒間有効化
                        w.magnet_timer = 10.0;
                        // 磁石パーティクル（黄）
                        w.particles.emit(px, py, 8, [1.0, 0.9, 0.2, 1.0]);
                    }
                }
                w.frame_events.push(FrameEvent::ItemPickup { item_kind: item_k as u8 });
                w.items.kill(i);
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
    // 最大の敵半径（Golem: 32px）を考慮してクエリ半径を広げる
    let bullet_query_r = BULLET_RADIUS + 32.0_f32;
    for bi in 0..bullet_len {
        if !w.bullets.alive[bi] {
            continue;
        }
        let dmg = w.bullets.damage[bi];
        // ダメージ 0 はエフェクト専用弾（Whip / Lightning）— 衝突判定をスキップ
        if dmg == 0 {
            continue;
        }
        let bx       = w.bullets.positions_x[bi];
        let by       = w.bullets.positions_y[bi];
        let piercing = w.bullets.piercing[bi];

        let nearby = w.collision.dynamic.query_nearby(bx, by, bullet_query_r);
        for ei in nearby {
            if !w.enemies.alive[ei] {
                continue;
            }
            let kind    = w.enemies.kinds[ei];
            let enemy_r = kind.radius();
            let hit_r   = BULLET_RADIUS + enemy_r;
            let ex  = w.enemies.positions_x[ei] + enemy_r;
            let ey  = w.enemies.positions_y[ei] + enemy_r;
            let ddx = bx - ex;
            let ddy = by - ey;
            if ddx * ddx + ddy * ddy < hit_r * hit_r {
                w.enemies.hp[ei] -= dmg as f32;
                if w.enemies.hp[ei] <= 0.0 {
                    let weapon_k = w.bullets.weapon_kind[bi];
                    w.enemies.kill(ei);
                    w.frame_events.push(FrameEvent::EnemyKilled {
                        enemy_kind:  kind as u8,
                        weapon_kind: weapon_k,
                    });
                    // ── Step 13: 敵撃破でスコア加算 ──────────────
                    // Step 18: 敵タイプに応じたスコア（経験値 × 2）
                    w.score += kind.exp_reward() * 2;
                    // ── Step 14/18: 経験値加算（タイプ別）────────
                    w.exp += kind.exp_reward();
                    if !w.level_up_pending {
                        let required = exp_required_for_next(w.level);
                        if w.exp >= required {
                            let new_lv = w.level + 1;
                            w.level_up_pending = true;
                            w.frame_events.push(FrameEvent::LevelUp { new_level: new_lv });
                        }
                    }
                    // ── Step 16/18: 敵タイプ別パーティクル ────────
                    let particle_color = match kind {
                        EnemyKind::Slime => [1.0, 0.5, 0.1, 1.0],
                        EnemyKind::Bat   => [0.7, 0.2, 0.9, 1.0],
                        EnemyKind::Golem => [0.6, 0.6, 0.6, 1.0],
                    };
                    w.particles.emit(ex, ey, 8, particle_color);
                    // ── Step 19: アイテムドロップ（1体につき最大1種類）──
                    // 0〜1%: 磁石、2〜6%: 回復ポーション、7〜100%: 経験値宝石
                    let roll = w.rng.next_u32() % 100;
                    let (item_kind, item_value) = if roll < 2 {
                        (ItemKind::Magnet, 0)
                    } else if roll < 7 {
                        (ItemKind::Potion, 20)
                    } else {
                        (ItemKind::Gem, kind.exp_reward())
                    };
                    w.items.spawn(ex, ey, item_kind, item_value);
                } else {
                    // ── Step 16: ヒット時黄色パーティクル ─────────
                    // ── Step 21: Fireball は炎色パーティクル ──────
                    let hit_color = if piercing {
                        [1.0, 0.4, 0.0, 1.0]  // 炎（橙赤）
                    } else {
                        [1.0, 0.9, 0.3, 1.0]  // 通常（黄）
                    };
                    w.particles.emit(ex, ey, 3, hit_color);
                }
                // 貫通弾は消えない、通常弾は消す
                if !piercing {
                    w.bullets.kill(bi);
                    break;
                }
            }
        }
    }

    // ── Step 24: ボス更新（Elixir が spawn_boss で生成したボスを毎フレーム動かす）
    {
        // 借用競合を避けるため、副作用データを先に収集する
        struct BossEffect {
            spawn_slimes:    bool,
            spawn_rocks:     bool,
            bat_dash:        bool,
            special_x:       f32,
            special_y:       f32,
            hurt_player:     bool,
            hurt_x:          f32,
            hurt_y:          f32,
            boss_damage:     f32,
            bullet_hits:     Vec<(usize, f32, bool)>,  // (bullet_idx, dmg, kill_bullet)
            boss_x:          f32,
            boss_y:          f32,
            boss_invincible: bool,
            boss_r:          f32,
            boss_exp_reward: u32,
            boss_hp_ref:     f32,
            boss_killed:     bool,
            exp_reward:      u32,
            kill_x:          f32,
            kill_y:          f32,
        }
        let mut eff = BossEffect {
            spawn_slimes: false, spawn_rocks: false, bat_dash: false,
            special_x: 0.0, special_y: 0.0,
            hurt_player: false, hurt_x: 0.0, hurt_y: 0.0,
            boss_damage: 0.0,
            bullet_hits: Vec::new(),
            boss_x: 0.0, boss_y: 0.0,
            boss_invincible: false, boss_r: 0.0, boss_exp_reward: 0, boss_hp_ref: 0.0,
            boss_killed: false, exp_reward: 0, kill_x: 0.0, kill_y: 0.0,
        };

        // ── フェーズ1: boss の移動・タイマー更新（boss のみを借用）──
        if w.boss.is_some() {
            // プレイヤー座標をコピーして boss 借用前に取得
            let px = w.player.x + PLAYER_RADIUS;
            let py = w.player.y + PLAYER_RADIUS;

            let boss = w.boss.as_mut().unwrap();

            // 無敵タイマー
            if boss.invincible_timer > 0.0 {
                boss.invincible_timer = (boss.invincible_timer - dt).max(0.0);
                if boss.invincible_timer <= 0.0 { boss.invincible = false; }
            }

            // 移動 AI
            match boss.kind {
                BossKind::SlimeKing | BossKind::StoneGolem => {
                    let ddx = px - boss.x;
                    let ddy = py - boss.y;
                    let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.001);
                    let spd = boss.kind.speed();
                    boss.x += (ddx / dist) * spd * dt;
                    boss.y += (ddy / dist) * spd * dt;
                }
                BossKind::BatLord => {
                    if boss.is_dashing {
                        boss.x += boss.dash_vx * dt;
                        boss.y += boss.dash_vy * dt;
                        boss.dash_timer -= dt;
                        if boss.dash_timer <= 0.0 {
                            boss.is_dashing = false;
                            boss.invincible = false;
                            boss.invincible_timer = 0.0;
                        }
                    } else {
                        let ddx = px - boss.x;
                        let ddy = py - boss.y;
                        let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.001);
                        boss.x += (ddx / dist) * boss.kind.speed() * dt;
                        boss.y += (ddy / dist) * boss.kind.speed() * dt;
                    }
                }
            }
            boss.x = boss.x.clamp(boss.kind.radius(), SCREEN_WIDTH  - boss.kind.radius());
            boss.y = boss.y.clamp(boss.kind.radius(), SCREEN_HEIGHT - boss.kind.radius());

            // 特殊行動タイマー
            boss.phase_timer -= dt;
            if boss.phase_timer <= 0.0 {
                boss.phase_timer = boss.kind.special_interval();
                match boss.kind {
                    BossKind::SlimeKing => {
                        eff.spawn_slimes = true;
                        eff.special_x = boss.x;
                        eff.special_y = boss.y;
                    }
                    BossKind::BatLord => {
                        let ddx = px - boss.x;
                        let ddy = py - boss.y;
                        let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.001);
                        boss.dash_vx = (ddx / dist) * 500.0;
                        boss.dash_vy = (ddy / dist) * 500.0;
                        boss.is_dashing = true;
                        boss.dash_timer = 0.6;
                        boss.invincible = true;
                        boss.invincible_timer = 0.6;
                        eff.bat_dash = true;
                        eff.special_x = boss.x;
                        eff.special_y = boss.y;
                    }
                    BossKind::StoneGolem => {
                        eff.spawn_rocks = true;
                        eff.special_x = boss.x;
                        eff.special_y = boss.y;
                    }
                }
            }

            // ボス vs プレイヤー接触ダメージ: フラグだけ立てる
            let boss_r = boss.kind.radius();
            let hit_r  = PLAYER_RADIUS + boss_r;
            let ddx = px - boss.x;
            let ddy = py - boss.y;
            if ddx * ddx + ddy * ddy < hit_r * hit_r {
                eff.hurt_player = true;
                eff.hurt_x = px;
                eff.hurt_y = py;
                eff.boss_damage = boss.kind.damage_per_sec();
            }

            // 弾丸 vs ボス: ヒット判定に必要なデータをコピー
            eff.boss_invincible = boss.invincible;
            eff.boss_r          = boss.kind.radius();
            eff.boss_exp_reward = boss.kind.exp_reward();
            eff.boss_x          = boss.x;
            eff.boss_y          = boss.y;
            eff.boss_hp_ref     = boss.hp;
        }
        // boss 借用をここで解放してから弾丸データにアクセス

        // 弾丸 vs ボス: boss 借用の外で処理
        if w.boss.is_some() && !eff.boss_invincible {
            let bullet_len = w.bullets.positions_x.len();
            for bi in 0..bullet_len {
                if !w.bullets.alive[bi] { continue; }
                let dmg = w.bullets.damage[bi];
                if dmg == 0 { continue; }
                let bx = w.bullets.positions_x[bi];
                let by = w.bullets.positions_y[bi];
                let hit_r2 = BULLET_RADIUS + eff.boss_r;
                let ddx2 = bx - eff.boss_x;
                let ddy2 = by - eff.boss_y;
                if ddx2 * ddx2 + ddy2 * ddy2 < hit_r2 * hit_r2 {
                    eff.bullet_hits.push((bi, dmg as f32, !w.bullets.piercing[bi]));
                }
            }
            // ダメージ適用
            let total_dmg: f32 = eff.bullet_hits.iter().map(|&(_, d, _)| d).sum();
            if total_dmg > 0.0 {
                if let Some(ref mut boss) = w.boss {
                    boss.hp -= total_dmg;
                    if boss.hp <= 0.0 {
                        eff.boss_killed = true;
                        eff.exp_reward  = eff.boss_exp_reward;
                        eff.kill_x      = boss.x;
                        eff.kill_y      = boss.y;
                    }
                }
            }
        }

        // ── フェーズ2: boss 借用を解放してから副作用を適用 ────────

        // プレイヤーダメージ
        if eff.hurt_player {
            if w.player.invincible_timer <= 0.0 && w.player.hp > 0.0 {
                let dmg = eff.boss_damage * dt;
                w.player.hp = (w.player.hp - dmg).max(0.0);
                w.player.invincible_timer = INVINCIBLE_DURATION;
                w.frame_events.push(FrameEvent::PlayerDamaged { damage: dmg });
                w.particles.emit(eff.hurt_x, eff.hurt_y, 8, [1.0, 0.15, 0.15, 1.0]);
            }
        }

        // 弾丸ヒットパーティクル & 弾丸消去
        if !eff.bullet_hits.is_empty() {
            w.particles.emit(eff.boss_x, eff.boss_y, 4, [1.0, 0.8, 0.2, 1.0]);
            for &(bi, _, kill_bullet) in &eff.bullet_hits {
                if kill_bullet { w.bullets.kill(bi); }
            }
        }

        // 特殊行動の副作用
        if eff.spawn_slimes {
            let positions: Vec<(f32, f32)> = (0..8).map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                (eff.special_x + angle.cos() * 120.0, eff.special_y + angle.sin() * 120.0)
            }).collect();
            w.enemies.spawn(&positions, EnemyKind::Slime);
            w.particles.emit(eff.special_x, eff.special_y, 16, [0.2, 1.0, 0.2, 1.0]);
        }
        if eff.spawn_rocks {
            for (dx_dir, dy_dir) in [(1.0_f32, 0.0_f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                w.bullets.spawn_ex(eff.special_x, eff.special_y, dx_dir * 200.0, dy_dir * 200.0, 50, 3.0, false, BULLET_KIND_ROCK, 0);
            }
            w.particles.emit(eff.special_x, eff.special_y, 10, [0.6, 0.6, 0.6, 1.0]);
        }
        if eff.bat_dash {
            w.particles.emit(eff.special_x, eff.special_y, 12, [0.8, 0.2, 1.0, 1.0]);
        }
        if eff.boss_killed {
            let boss_k = w.boss.as_ref().map(|b| b.kind as u8).unwrap_or(0);
            w.frame_events.push(FrameEvent::BossDefeated { boss_kind: boss_k });
            w.score += eff.exp_reward * 2;
            w.exp   += eff.exp_reward;
            if !w.level_up_pending {
                let required = exp_required_for_next(w.level);
                if w.exp >= required {
                    let new_lv = w.level + 1;
                    w.level_up_pending = true;
                    w.frame_events.push(FrameEvent::LevelUp { new_level: new_lv });
                }
            }
            w.particles.emit(eff.kill_x, eff.kill_y, 40, [1.0, 0.5, 0.0, 1.0]);
            for _ in 0..10 {
                let ox = (w.rng.next_f32() - 0.5) * 200.0;
                let oy = (w.rng.next_f32() - 0.5) * 200.0;
                w.items.spawn(eff.kill_x + ox, eff.kill_y + oy, ItemKind::Gem, eff.exp_reward / 10);
            }
            w.boss = None;
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

    Ok(w.frame_id)
}

/// Step 26: フレームイベントを取り出してクリアする（毎フレーム GameLoop が呼ぶ）
/// 戻り値: [{:enemy_killed, enemy_kind, weapon_kind}, {:player_damaged, damage_x100, 0}, ...]
#[rustler::nif]
fn drain_frame_events(world: ResourceArc<GameWorld>) -> NifResult<Vec<(Atom, u8, u8)>> {
    let mut w = world.0.write().map_err(|_| lock_poisoned_err())?;
    let events = w.frame_events
        .drain(..)
        .map(|e| match e {
            FrameEvent::EnemyKilled { enemy_kind, weapon_kind } =>
                (enemy_killed(), enemy_kind, weapon_kind),
            FrameEvent::PlayerDamaged { damage } =>
                (player_damaged(), (damage * 100.0).min(255.0) as u8, 0),
            FrameEvent::LevelUp { new_level } =>
                (level_up_event(), new_level.min(255) as u8, 0),
            FrameEvent::ItemPickup { item_kind } =>
                (item_pickup(), item_kind, 0),
            FrameEvent::BossDefeated { boss_kind } =>
                (boss_defeated(), boss_kind, 0),
        })
        .collect();
    Ok(events)
}

/// プレイヤー座標を返す（Step 8）
#[rustler::nif]
fn get_player_pos(world: ResourceArc<GameWorld>) -> NifResult<(f64, f64)> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok((w.player.x as f64, w.player.y as f64))
}

/// プレイヤー HP を返す（Step 10）
#[rustler::nif]
fn get_player_hp(world: ResourceArc<GameWorld>) -> NifResult<f64> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.player.hp as f64)
}

/// 描画データを返す: [{x, y, kind}] のリスト
/// kind: 0=player, 1=slime, 2=bat, 3=golem, 4=bullet,
///       11=SlimeKing, 12=BatLord, 13=StoneGolem, 14=rock_bullet
#[rustler::nif]
fn get_render_data(world: ResourceArc<GameWorld>) -> NifResult<Vec<(f32, f32, u8)>> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    let mut result = Vec::with_capacity(1 + w.enemies.len() + w.bullets.len() + 1);
    result.push((w.player.x, w.player.y, 0u8));
    // Step 24: ボスを描画（中心座標からスプライト左上に変換）
    if let Some(ref boss) = w.boss {
        let boss_sprite_size = match boss.kind {
            BossKind::StoneGolem => 128.0,
            _ => 96.0,
        };
        result.push((
            boss.x - boss_sprite_size / 2.0,
            boss.y - boss_sprite_size / 2.0,
            boss.kind.render_kind(),
        ));
    }
    for i in 0..w.enemies.len() {
        if w.enemies.alive[i] {
            result.push((
                w.enemies.positions_x[i],
                w.enemies.positions_y[i],
                w.enemies.kinds[i].render_kind(),
            ));
        }
    }
    for i in 0..w.bullets.len() {
        if w.bullets.alive[i] {
            result.push((w.bullets.positions_x[i], w.bullets.positions_y[i], w.bullets.render_kind[i]));
        }
    }
    Ok(result)
}

/// パーティクル描画データを返す: [(x, y, r, g, b, alpha, size)]
#[rustler::nif]
fn get_particle_data(world: ResourceArc<GameWorld>) -> NifResult<Vec<(f32, f32, f32, f32, f32, f32, f32)>> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
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
    Ok(result)
}

/// 現在飛んでいる弾丸数を返す（Step 11）
#[rustler::nif]
fn get_bullet_count(world: ResourceArc<GameWorld>) -> NifResult<usize> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.bullets.count)
}

/// 直近フレームの物理ステップ処理時間をミリ秒で返す（Step 12）
#[rustler::nif]
fn get_frame_time_ms(world: ResourceArc<GameWorld>) -> NifResult<f64> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.last_frame_time_ms)
}

/// 現在生存している敵の数を返す（Step 12）
#[rustler::nif]
fn get_enemy_count(world: ResourceArc<GameWorld>) -> NifResult<usize> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.enemies.count)
}

/// HUD データを一括取得（Step 13）
/// 戻り値: (hp, max_hp, score, elapsed_seconds)
#[rustler::nif]
fn get_hud_data(world: ResourceArc<GameWorld>) -> NifResult<(f64, f64, u32, f64)> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok((
        w.player.hp        as f64,
        w.player_max_hp    as f64,
        w.score,
        w.elapsed_seconds  as f64,
    ))
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
fn get_level_up_data(world: ResourceArc<GameWorld>) -> NifResult<(u32, u32, bool, u32)> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    let exp_to_next = exp_required_for_next(w.level).saturating_sub(w.exp);
    Ok((w.exp, w.level, w.level_up_pending, exp_to_next))
}

/// 装備中の武器スロット情報を返す（Step 17）
/// 戻り値: [(weapon_name, level)] のリスト
#[rustler::nif]
fn get_weapon_levels(world: ResourceArc<GameWorld>) -> NifResult<Vec<(String, u32)>> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.weapon_slots.iter()
        .map(|s| (s.kind.name().to_string(), s.level))
        .collect())
}

/// 武器を追加またはレベルアップし、レベルアップ待機を解除する（Step 17/21）
/// weapon_name: "magic_wand" | "axe" | "cross" | "whip" | "fireball" | "lightning"
/// 同じ武器を選んだ場合はレベルアップ（最大 Lv.8）
/// 新規武器は最大 6 スロットまで追加可能
#[rustler::nif]
fn add_weapon(world: ResourceArc<GameWorld>, weapon_name: &str) -> NifResult<Atom> {
    let mut w = world.0.write().map_err(|_| lock_poisoned_err())?;

    let kind = match weapon_name {
        "magic_wand" => WeaponKind::MagicWand,
        "axe"        => WeaponKind::Axe,
        "cross"      => WeaponKind::Cross,
        "whip"       => WeaponKind::Whip,
        "fireball"   => WeaponKind::Fireball,
        "lightning"  => WeaponKind::Lightning,
        _            => WeaponKind::MagicWand,
    };

    // 同じ武器を選んだ場合はレベルアップ
    if let Some(slot) = w.weapon_slots.iter_mut().find(|s| s.kind == kind) {
        slot.level = (slot.level + 1).min(weapon::MAX_WEAPON_LEVEL);
    } else if w.weapon_slots.len() < MAX_WEAPON_SLOTS {
        w.weapon_slots.push(WeaponSlot::new(kind));
    }
    // Slots full + new weapon: no-op (Elixir-side generate_weapon_choices must not offer this)

    // exp は累積値で管理するためリセットしない
    w.complete_level_up();

    Ok(ok())
}

/// 武器選択をスキップしてレベルアップ待機を解除する
/// 全武器がMaxLvの場合など、選択肢がない状態で呼び出す
#[rustler::nif]
fn skip_level_up(world: ResourceArc<GameWorld>) -> NifResult<Atom> {
    let mut w = world.0.write().map_err(|_| lock_poisoned_err())?;
    w.complete_level_up();
    Ok(ok())
}

// ─── Step 19: アイテム関連 NIF ─────────────────────────────────

/// アイテム描画データを返す: [(x, y, kind)] kind: 5=gem, 6=potion, 7=magnet
#[rustler::nif]
fn get_item_data(world: ResourceArc<GameWorld>) -> NifResult<Vec<(f32, f32, u8)>> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    let mut result = Vec::with_capacity(w.items.count);
    for i in 0..w.items.len() {
        if w.items.alive[i] {
            result.push((
                w.items.positions_x[i],
                w.items.positions_y[i],
                w.items.kinds[i].render_kind(),
            ));
        }
    }
    Ok(result)
}

/// 磁石エフェクトの残り時間（秒）を返す
#[rustler::nif]
fn get_magnet_timer(world: ResourceArc<GameWorld>) -> NifResult<f64> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.magnet_timer as f64)
}

// ─── Step 24: ボス関連 NIF ─────────────────────────────────────

/// ボスをスポーンする（Elixir 側から呼び出す）
/// kind: :slime_king | :bat_lord | :stone_golem
/// スポーン位置はプレイヤーの右 600px
#[rustler::nif]
fn spawn_boss(world: ResourceArc<GameWorld>, kind: Atom) -> NifResult<Atom> {
    let mut w = world.0.write().map_err(|_| lock_poisoned_err())?;
    if w.boss.is_some() { return Ok(ok()); }
    if let Some(boss_kind) = BossKind::from_atom(kind) {
        let px = w.player.x + PLAYER_RADIUS;
        let py = w.player.y + PLAYER_RADIUS;
        let bx = (px + 600.0).min(SCREEN_WIDTH  - boss_kind.radius());
        let by = py.clamp(boss_kind.radius(), SCREEN_HEIGHT - boss_kind.radius());
        w.boss = Some(BossState::new(boss_kind, bx, by));
    }
    Ok(ok())
}

/// ボスの状態を返す: {status_atom, hp, max_hp}
/// status_atom: :alive | :none
#[rustler::nif]
fn get_boss_info(world: ResourceArc<GameWorld>) -> NifResult<(Atom, f64, f64)> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(match &w.boss {
        Some(boss) => (alive(), boss.hp as f64, boss.max_hp as f64),
        None       => (none(),  0.0,            0.0),
    })
}

/// プレイヤーが死亡しているかを返す（HP == 0 で true）
#[rustler::nif]
fn is_player_dead(world: ResourceArc<GameWorld>) -> NifResult<bool> {
    let w = world.0.read().map_err(|_| lock_poisoned_err())?;
    Ok(w.player.hp <= 0.0)
}

/// エリート敵をスポーンする（通常敵の hp_multiplier 倍の HP を持つ）
/// kind: :slime | :bat | :golem
/// hp_multiplier: 1.0 = 通常、3.0 = エリート（HP 3 倍）
#[rustler::nif]
fn spawn_elite_enemy(world: ResourceArc<GameWorld>, kind: Atom, count: usize, hp_multiplier: f64) -> NifResult<Atom> {
    let mut w = world.0.write().map_err(|_| lock_poisoned_err())?;
    let enemy_kind = EnemyKind::from_atom(kind);
    let positions: Vec<(f32, f32)> = (0..count)
        .map(|_| spawn_position_outside(&mut w.rng, SCREEN_WIDTH, SCREEN_HEIGHT))
        .collect();
    // 通常スポーン後に HP を倍率で上書き
    let before_len = w.enemies.positions_x.len();
    w.enemies.spawn(&positions, enemy_kind);
    let after_len = w.enemies.positions_x.len();
    let base_hp = enemy_kind.max_hp() * hp_multiplier as f32;
    // 新規追加分と再利用スロット分の両方に倍率を適用
    // 再利用スロットは spawn 内で hp をリセット済みのため、全 alive スロットを走査して
    // 直近 count 体分（最後に alive になったもの）に倍率を掛ける
    // 簡易実装: 末尾 count 体の alive スロットを倍率適用
    let mut applied = 0;
    for i in (0..after_len).rev() {
        if applied >= count { break; }
        if w.enemies.alive[i] && w.enemies.kinds[i] == enemy_kind {
            // 新規追加分（before_len 以降）または最近 alive になったスロット
            if i >= before_len || (w.enemies.hp[i] - enemy_kind.max_hp()).abs() < 0.01 {
                w.enemies.hp[i] = base_hp;
                applied += 1;
            }
        }
    }
    Ok(ok())
}

// ─── ローダー ─────────────────────────────────────────────────

#[allow(non_local_definitions)]
fn load(env: rustler::Env, _: rustler::Term) -> bool {
    let _ = rustler::resource!(GameWorld, env);
    // アトムを NIF ロード時に事前登録して、比較が確実に動作するようにする
    let _ = ok();
    let _ = slime();
    let _ = bat();
    let _ = golem();
    true
}

rustler::init!("Elixir.Game.NifBridge", load = load);

