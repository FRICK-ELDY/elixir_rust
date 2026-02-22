/// Standalone rendering binary.
/// Runs the full game loop in pure Rust without Elixir/NIF.
/// Used for renderer development and visual testing.
mod audio;
mod constants;
mod item;
mod renderer;
mod physics;
mod weapon;

// ─── Step 22: 音声ファイルをバイナリに埋め込む ──────────────────────
// assets/audio/ 以下の WAV ファイルが存在しない場合はコンパイルエラーになる。
// `cargo run` 前に `python assets/audio/gen_audio.py` を実行すること。
static BGM_BYTES:          &[u8] = include_bytes!("../../../assets/audio/bgm.wav");
static HIT_BYTES:          &[u8] = include_bytes!("../../../assets/audio/hit.wav");
static DEATH_BYTES:        &[u8] = include_bytes!("../../../assets/audio/death.wav");
static LEVEL_UP_BYTES:     &[u8] = include_bytes!("../../../assets/audio/level_up.wav");
static PLAYER_HURT_BYTES:  &[u8] = include_bytes!("../../../assets/audio/player_hurt.wav");
static ITEM_PICKUP_BYTES:  &[u8] = include_bytes!("../../../assets/audio/item_pickup.wav");

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use audio::AudioManager;

use constants::{
    BULLET_LIFETIME, BULLET_RADIUS, BULLET_SPEED,
    CAMERA_LERP_SPEED, CELL_SIZE, ENEMY_SEPARATION_FORCE,
    ENEMY_SEPARATION_RADIUS, INVINCIBLE_DURATION,
    MAP_HEIGHT, MAP_WIDTH,
    MAX_ENEMIES, PLAYER_RADIUS, PLAYER_SIZE, PLAYER_SPEED,
    SCREEN_HEIGHT, SCREEN_WIDTH, WAVES,
};
use item::{ItemKind, ItemWorld};
use renderer::{BossHudInfo, HudData, Renderer};
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
    /// Step 23: アニメーション FPS
    fn anim_fps(self) -> f32 {
        match self { Self::Slime => 6.0, Self::Bat => 12.0, Self::Golem => 4.0 }
    }
    /// Step 23: アニメーションフレーム数
    fn frame_count(self) -> u8 {
        match self { Self::Slime => 4, Self::Bat => 2, Self::Golem => 2 }
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
// ─── Step 24: ボスエネミー ─────────────────────────────────────

/// ボスの種類
#[derive(Clone, Copy, PartialEq, Debug)]
enum BossKind {
    SlimeKing,   // 3 分: スライムを召喚
    BatLord,     // 6 分: 高速突進 + 無敵
    StoneGolem,  // 9 分: 岩を投げる（範囲攻撃）
}

impl BossKind {
    fn max_hp(self) -> f32 {
        match self { Self::SlimeKing => 1000.0, Self::BatLord => 2000.0, Self::StoneGolem => 5000.0 }
    }
    fn speed(self) -> f32 {
        match self { Self::SlimeKing => 60.0, Self::BatLord => 200.0, Self::StoneGolem => 30.0 }
    }
    fn radius(self) -> f32 {
        match self { Self::SlimeKing => 48.0, Self::BatLord => 48.0, Self::StoneGolem => 64.0 }
    }
    fn exp_reward(self) -> u32 {
        match self { Self::SlimeKing => 200, Self::BatLord => 400, Self::StoneGolem => 800 }
    }
    fn damage_per_sec(self) -> f32 {
        match self { Self::SlimeKing => 30.0, Self::BatLord => 50.0, Self::StoneGolem => 80.0 }
    }
    fn name(self) -> &'static str {
        match self { Self::SlimeKing => "Slime King", Self::BatLord => "Bat Lord", Self::StoneGolem => "Stone Golem" }
    }
    /// render_kind（renderer の kind 番号）
    fn render_kind(self) -> u8 {
        match self { Self::SlimeKing => 11, Self::BatLord => 12, Self::StoneGolem => 13 }
    }
    /// 特殊行動のインターバル（秒）
    fn special_interval(self) -> f32 {
        match self { Self::SlimeKing => 5.0, Self::BatLord => 4.0, Self::StoneGolem => 6.0 }
    }
}

/// ボスの状態
struct BossState {
    kind:           BossKind,
    x:              f32,
    y:              f32,
    hp:             f32,
    max_hp:         f32,
    phase_timer:    f32,   // 特殊行動タイマー
    invincible:     bool,  // BatLord の無敵フラグ
    invincible_timer: f32, // 無敵の残り時間
    alive:          bool,
    /// BatLord: 突進中フラグ
    is_dashing:     bool,
    dash_timer:     f32,
    dash_vx:        f32,
    dash_vy:        f32,
}

impl BossState {
    fn new(kind: BossKind, x: f32, y: f32) -> Self {
        let max_hp = kind.max_hp();
        Self {
            kind,
            x,
            y,
            hp: max_hp,
            max_hp,
            phase_timer: kind.special_interval(),
            invincible: false,
            invincible_timer: 0.0,
            alive: true,
            is_dashing: false,
            dash_timer: 0.0,
            dash_vx: 0.0,
            dash_vy: 0.0,
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
    /// Step 23: アニメーションタイマー（秒）
    anim_timer: f32,
    /// Step 23: 現在のアニメーションフレーム番号（0〜3）
    anim_frame: u8,
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
    /// Step 23: アニメーションタイマー（秒）
    anim_timers:  Vec<f32>,
    /// Step 23: 現在のアニメーションフレーム番号
    anim_frames:  Vec<u8>,
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
            anim_timers:  Vec::new(),
            anim_frames:  Vec::new(),
        }
    }
    fn spawn(&mut self, positions: &[(f32, f32)], kind: EnemyKind) {
        let max_hp = kind.max_hp();
        for &(x, y) in positions {
            let slot = self.alive.iter().position(|&a| !a);
            if let Some(i) = slot {
                self.positions_x[i]  = x;
                self.positions_y[i]  = y;
                self.hp[i]           = max_hp;
                self.alive[i]        = true;
                self.kinds[i]        = kind;
                self.anim_timers[i]  = 0.0;
                self.anim_frames[i]  = 0;
            } else {
                self.positions_x.push(x);
                self.positions_y.push(y);
                self.hp.push(max_hp);
                self.alive.push(true);
                self.kinds.push(kind);
                self.sep_x.push(0.0);
                self.sep_y.push(0.0);
                self.anim_timers.push(0.0);
                self.anim_frames.push(0);
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

const BULLET_KIND_NORMAL:   u8 = 4;
const BULLET_KIND_FIREBALL: u8 = 8;
// 11=SlimeKing, 12=BatLord, 13=StoneGolem（ボス render_kind と共有）
const BULLET_KIND_ROCK:     u8 = 14; // StoneGolem の岩弾

struct BulletWorld {
    positions_x:  Vec<f32>,
    positions_y:  Vec<f32>,
    velocities_x: Vec<f32>,
    velocities_y: Vec<f32>,
    damage:       Vec<i32>,
    lifetime:     Vec<f32>,
    alive:        Vec<bool>,
    /// true の弾丸は敵に当たっても消えずに貫通する（Fireball 用）
    piercing:     Vec<bool>,
    /// 描画種別（BULLET_KIND_* 定数）
    render_kind:  Vec<u8>,
    count:        usize,
}

impl BulletWorld {
    fn new() -> Self {
        Self { positions_x: Vec::new(), positions_y: Vec::new(), velocities_x: Vec::new(), velocities_y: Vec::new(), damage: Vec::new(), lifetime: Vec::new(), alive: Vec::new(), piercing: Vec::new(), render_kind: Vec::new(), count: 0 }
    }
    fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32, dmg: i32) {
        self.spawn_ex(x, y, vx, vy, dmg, false, BULLET_KIND_NORMAL);
    }
    fn spawn_piercing(&mut self, x: f32, y: f32, vx: f32, vy: f32, dmg: i32) {
        self.spawn_ex(x, y, vx, vy, dmg, true, BULLET_KIND_FIREBALL);
    }
    /// ダメージ 0・短命の表示専用エフェクト弾を生成する（Whip / Lightning 用）
    fn spawn_effect(&mut self, x: f32, y: f32, lifetime: f32, render_kind: u8) {
        // lifetime を直接設定するため spawn_ex の後に上書きする
        let slot = self.alive.iter().position(|&a| !a);
        if let Some(i) = slot {
            self.positions_x[i]  = x;
            self.positions_y[i]  = y;
            self.velocities_x[i] = 0.0;
            self.velocities_y[i] = 0.0;
            self.damage[i]       = 0;
            self.lifetime[i]     = lifetime;
            self.alive[i]        = true;
            self.piercing[i]     = false;
            self.render_kind[i]  = render_kind;
        } else {
            self.positions_x.push(x);
            self.positions_y.push(y);
            self.velocities_x.push(0.0);
            self.velocities_y.push(0.0);
            self.damage.push(0);
            self.lifetime.push(lifetime);
            self.alive.push(true);
            self.piercing.push(false);
            self.render_kind.push(render_kind);
        }
        self.count += 1;
    }
    fn spawn_ex(&mut self, x: f32, y: f32, vx: f32, vy: f32, dmg: i32, piercing: bool, render_kind: u8) {
        let slot = self.alive.iter().position(|&a| !a);
        if let Some(i) = slot {
            self.positions_x[i]  = x;
            self.positions_y[i]  = y;
            self.velocities_x[i] = vx;
            self.velocities_y[i] = vy;
            self.damage[i]       = dmg;
            self.lifetime[i]     = BULLET_LIFETIME;
            self.alive[i]        = true;
            self.piercing[i]     = piercing;
            self.render_kind[i]  = render_kind;
        } else {
            self.positions_x.push(x);
            self.positions_y.push(y);
            self.velocities_x.push(vx);
            self.velocities_y.push(vy);
            self.damage.push(dmg);
            self.lifetime.push(BULLET_LIFETIME);
            self.alive.push(true);
            self.piercing.push(piercing);
            self.render_kind.push(render_kind);
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
    // Step 19: アイテム
    items:            ItemWorld,
    magnet_timer:     f32,
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
    // Step 20: カメラ（プレイヤー追従スクロール）
    camera_x:         f32,
    camera_y:         f32,
    /// 実際のウィンドウサイズ（リサイズ対応）
    screen_w:         f32,
    screen_h:         f32,
    // Step 24: ボスエネミー
    boss:             Option<BossState>,
    /// 次に出現するボスのインデックス（0=SlimeKing, 1=BatLord, 2=StoneGolem）
    next_boss_index:  usize,
    /// ボス出現通知フラグ（1 フレームだけ true になる）
    boss_spawned:     bool,
}

/// Step 22: 1 フレーム中に発生した音声イベント
#[derive(Default)]
struct SoundEvents {
    pub enemy_hit:    bool,
    pub enemy_death:  bool,
    pub level_up:     bool,
    pub player_hurt:  bool,
    pub item_pickup:  bool,
    /// Step 24: ボス出現
    pub boss_spawn:   bool,
}

impl GameWorld {
    fn new() -> Self {
        // プレイヤーはマップ中央からスタート
        let start_x = MAP_WIDTH  / 2.0 - PLAYER_SIZE / 2.0;
        let start_y = MAP_HEIGHT / 2.0 - PLAYER_SIZE / 2.0;
        // カメラ初期位置: プレイヤーが画面中央に来るように（初期ウィンドウサイズ基準）
        let cam_x = start_x + PLAYER_SIZE / 2.0 - SCREEN_WIDTH  / 2.0;
        let cam_y = start_y + PLAYER_SIZE / 2.0 - SCREEN_HEIGHT / 2.0;
        Self {
            player: PlayerState {
                x: start_x,
                y: start_y,
                input_dx: 0.0, input_dy: 0.0,
                hp: 100.0, max_hp: 100.0,
                invincible_timer: 0.0,
                anim_timer: 0.0,
                anim_frame: 0,
            },
            enemies:          EnemyWorld::new(),
            bullets:          BulletWorld::new(),
            particles:        ParticleWorld::new(67890),
            items:            ItemWorld::new(),
            magnet_timer:     0.0,
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
            camera_x:         cam_x,
            camera_y:         cam_y,
            screen_w:         SCREEN_WIDTH,
            screen_h:         SCREEN_HEIGHT,
            boss:             None,
            next_boss_index:  0,
            boss_spawned:     false,
        }
    }

    /// ウィンドウリサイズ時に画面サイズを更新する（Step 20）
    fn on_resize(&mut self, width: u32, height: u32) {
        self.screen_w = width  as f32;
        self.screen_h = height as f32;
    }

    /// ゲームを 1 ステップ進め、このフレームで発生した音声イベントを返す。
    fn step(&mut self, dt: f32) -> SoundEvents {
        let mut se = SoundEvents::default();

        // レベルアップ中はゲームを一時停止（プレイヤーがボタンを選ぶまで待つ）
        if self.level_up_pending {
            return se;
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
        // Step 20: マップ境界内に制限
        self.player.x = self.player.x.clamp(0.0, MAP_WIDTH  - PLAYER_SIZE);
        self.player.y = self.player.y.clamp(0.0, MAP_HEIGHT - PLAYER_SIZE);

        let px = self.player.x + PLAYER_RADIUS;
        let py = self.player.y + PLAYER_RADIUS;

        // Step 20: カメラの滑らかな追従（lerp）
        // 実際のウィンドウサイズを使うことでリサイズ後も中央追従が正しく動作する
        let sw = self.screen_w;
        let sh = self.screen_h;
        let target_cam_x = px - sw / 2.0;
        let target_cam_y = py - sh / 2.0;
        // マップ端でカメラを止める
        let max_cam_x = (MAP_WIDTH  - sw).max(0.0);
        let max_cam_y = (MAP_HEIGHT - sh).max(0.0);
        let target_cam_x = target_cam_x.clamp(0.0, max_cam_x);
        let target_cam_y = target_cam_y.clamp(0.0, max_cam_y);
        let lerp_t = 1.0 - (-CAMERA_LERP_SPEED * dt).exp();
        self.camera_x += (target_cam_x - self.camera_x) * lerp_t;
        self.camera_y += (target_cam_y - self.camera_y) * lerp_t;

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

        // Step 23: プレイヤーアニメーション更新（歩行中のみ進める）
        {
            const PLAYER_ANIM_FPS: f32 = 8.0;
            const PLAYER_ANIM_INTERVAL: f32 = 1.0 / PLAYER_ANIM_FPS;
            let is_moving = self.player.input_dx * self.player.input_dx
                + self.player.input_dy * self.player.input_dy > 0.0001;
            if is_moving {
                self.player.anim_timer += dt;
                if self.player.anim_timer >= PLAYER_ANIM_INTERVAL {
                    self.player.anim_timer -= PLAYER_ANIM_INTERVAL;
                    self.player.anim_frame = (self.player.anim_frame + 1) % 4;
                }
            } else {
                self.player.anim_frame = 0;
                self.player.anim_timer = 0.0;
            }
        }

        // Step 23: 敵アニメーション更新
        {
            let elen = self.enemies.len();
            for i in 0..elen {
                if !self.enemies.alive[i] { continue; }
                let interval = 1.0 / self.enemies.kinds[i].anim_fps();
                self.enemies.anim_timers[i] += dt;
                if self.enemies.anim_timers[i] >= interval {
                    self.enemies.anim_timers[i] -= interval;
                    let max_frame = self.enemies.kinds[i].frame_count();
                    self.enemies.anim_frames[i] = (self.enemies.anim_frames[i] + 1) % max_frame;
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
                    se.player_hurt = true;
                }
            }
        }

        // Step 17/21: 武器スロット発射処理（レベルに応じたクールダウン・ダメージ・弾数）
        // プレイヤーの移動方向（Whip の向き計算用）
        let facing_angle = {
            let fdx = self.player.input_dx;
            let fdy = self.player.input_dy;
            if fdx * fdx + fdy * fdy > 0.0001 { fdy.atan2(fdx) } else { 0.0_f32 }
        };

        let slot_count = self.weapon_slots.len();
        for si in 0..slot_count {
            self.weapon_slots[si].cooldown_timer = (self.weapon_slots[si].cooldown_timer - dt).max(0.0);
            if self.weapon_slots[si].cooldown_timer > 0.0 { continue; }

            let cd     = self.weapon_slots[si].effective_cooldown();
            let dmg    = self.weapon_slots[si].effective_damage();
            let level  = self.weapon_slots[si].level;
            let bcount = self.weapon_slots[si].bullet_count();
            let kind   = self.weapon_slots[si].kind;

            match kind {
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
                // ── Step 21: Whip ──────────────────────────────────────────
                WeaponKind::Whip => {
                    let whip_range = kind.whip_range(level);
                    let whip_half_angle = std::f32::consts::PI * 0.3;
                    // facing_angle 方向の中間点にエフェクト弾を生成（kind=10: 黄緑の横長楕円）
                    let eff_x = px + facing_angle.cos() * whip_range * 0.5;
                    let eff_y = py + facing_angle.sin() * whip_range * 0.5;
                    self.bullets.spawn_effect(eff_x, eff_y, 0.12, 10);
                    // 空間ハッシュで範囲内の候補のみ取得し、全敵ループを回避
                    let whip_range_sq = whip_range * whip_range;
                    let candidates = self.collision.dynamic.query_nearby(px, py, whip_range);
                    for ei in candidates {
                        if !self.enemies.alive[ei] { continue; }
                        let ex = self.enemies.positions_x[ei];
                        let ey = self.enemies.positions_y[ei];
                        let ddx = ex - px;
                        let ddy = ey - py;
                        // sqrt を避けて二乗比較で正確な円形クリップ
                        if ddx * ddx + ddy * ddy > whip_range_sq { continue; }
                        let angle = ddy.atan2(ddx);
                        let mut diff = angle - facing_angle;
                        if diff >  std::f32::consts::PI { diff -= std::f32::consts::TAU; }
                        if diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
                        if diff.abs() < whip_half_angle {
                            let enemy_r = self.enemies.kinds[ei].radius();
                            let hit_x = ex + enemy_r;
                            let hit_y = ey + enemy_r;
                            self.enemies.hp[ei] -= dmg as f32;
                            if self.enemies.hp[ei] <= 0.0 {
                                self.enemies.kill(ei);
                                let kind_e = self.enemies.kinds[ei];
                                self.score += kind_e.exp_reward() * 2;
                                self.exp   += kind_e.exp_reward();
                                let prev_pending = self.level_up_pending;
                                self.check_level_up();
                                if self.level_up_pending && !prev_pending { se.level_up = true; }
                                let pc = match kind_e {
                                    EnemyKind::Slime => [1.0, 0.5, 0.1, 1.0],
                                    EnemyKind::Bat   => [0.7, 0.2, 0.9, 1.0],
                                    EnemyKind::Golem => [0.6, 0.6, 0.6, 1.0],
                                };
                                self.particles.emit(hit_x, hit_y, 8, pc);
                                se.enemy_death = true;
                                let roll = self.rng.next_u32() % 100;
                                let (item_kind, item_value) = if roll < 2 {
                                    (ItemKind::Magnet, 0)
                                } else if roll < 7 {
                                    (ItemKind::Potion, 20)
                                } else {
                                    (ItemKind::Gem, kind_e.exp_reward())
                                };
                                self.items.spawn(hit_x, hit_y, item_kind, item_value);
                            } else {
                                self.particles.emit(hit_x, hit_y, 3, [1.0, 0.6, 0.1, 1.0]);
                                se.enemy_hit = true;
                            }
                        }
                    }
                    // Step 24: Whip vs ボス
                    if let Some(ref mut boss) = self.boss {
                        if boss.alive && !boss.invincible {
                            let ddx = boss.x - px;
                            let ddy = boss.y - py;
                            let whip_range_sq = whip_range * whip_range;
                            if ddx * ddx + ddy * ddy <= whip_range_sq {
                                let angle = ddy.atan2(ddx);
                                let mut diff = angle - facing_angle;
                                if diff >  std::f32::consts::PI { diff -= std::f32::consts::TAU; }
                                if diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
                                if diff.abs() < whip_half_angle {
                                    boss.hp -= dmg as f32;
                                    self.particles.emit(boss.x, boss.y, 4, [1.0, 0.8, 0.2, 1.0]);
                                    se.enemy_hit = true;
                                }
                            }
                        }
                    }
                    self.weapon_slots[si].cooldown_timer = cd;
                }
                // ── Step 21: Fireball ──────────────────────────────────────
                WeaponKind::Fireball => {
                    if let Some(ti) = self.find_nearest_enemy(px, py) {
                        let target_r = self.enemies.kinds[ti].radius();
                        let tx  = self.enemies.positions_x[ti] + target_r;
                        let ty  = self.enemies.positions_y[ti] + target_r;
                        let bdx = tx - px;
                        let bdy = ty - py;
                        let base_angle = bdy.atan2(bdx);
                        let vx = base_angle.cos() * BULLET_SPEED;
                        let vy = base_angle.sin() * BULLET_SPEED;
                        self.bullets.spawn_piercing(px, py, vx, vy, dmg);
                        self.weapon_slots[si].cooldown_timer = cd;
                    }
                }
                // ── Step 21: Lightning ─────────────────────────────────────
                WeaponKind::Lightning => {
                    let chain_count = kind.lightning_chain_count(level);
                    // chain_count は最大 6 程度と小さいため Vec で十分（HashSet 不要）
                    let mut hit_vec: Vec<usize> = Vec::with_capacity(chain_count);
                    let mut current = self.find_nearest_enemy(px, py);
                    #[allow(unused_assignments)]
                    let mut next_search_x = px;
                    #[allow(unused_assignments)]
                    let mut next_search_y = py;
                    for _ in 0..chain_count {
                        if let Some(ei) = current {
                            let enemy_r = self.enemies.kinds[ei].radius();
                            let hit_x = self.enemies.positions_x[ei] + enemy_r;
                            let hit_y = self.enemies.positions_y[ei] + enemy_r;
                            self.enemies.hp[ei] -= dmg as f32;
                            // 電撃エフェクト弾（kind=9: 水色の電撃球）+ パーティクル
                            self.bullets.spawn_effect(hit_x, hit_y, 0.10, 9);
                            self.particles.emit(hit_x, hit_y, 5, [0.3, 0.8, 1.0, 1.0]);
                            if self.enemies.hp[ei] <= 0.0 {
                                self.enemies.kill(ei);
                                let kind_e = self.enemies.kinds[ei];
                                self.score += kind_e.exp_reward() * 2;
                                self.exp   += kind_e.exp_reward();
                                let prev_pending = self.level_up_pending;
                                self.check_level_up();
                                if self.level_up_pending && !prev_pending { se.level_up = true; }
                                se.enemy_death = true;
                                let roll = self.rng.next_u32() % 100;
                                let (item_kind, item_value) = if roll < 2 {
                                    (ItemKind::Magnet, 0)
                                } else if roll < 7 {
                                    (ItemKind::Potion, 20)
                                } else {
                                    (ItemKind::Gem, kind_e.exp_reward())
                                };
                                self.items.spawn(hit_x, hit_y, item_kind, item_value);
                            } else {
                                se.enemy_hit = true;
                            }
                            hit_vec.push(ei);
                            next_search_x = hit_x;
                            next_search_y = hit_y;
                            // 次のターゲット: 現在位置から最も近い未ヒット敵
                            current = {
                                let mut min_d = f32::MAX;
                                let mut next = None;
                                for i in 0..self.enemies.len() {
                                    if !self.enemies.alive[i] || hit_vec.contains(&i) { continue; }
                                    let dx = self.enemies.positions_x[i] - next_search_x;
                                    let dy = self.enemies.positions_y[i] - next_search_y;
                                    let d  = dx * dx + dy * dy;
                                    if d < min_d { min_d = d; next = Some(i); }
                                }
                                next
                            };
                        } else {
                            break;
                        }
                    }
                    // Step 24: Lightning vs ボス（チェーン先としてボスを含める）
                    if let Some(ref mut boss) = self.boss {
                        if boss.alive && !boss.invincible {
                            let ddx = boss.x - px;
                            let ddy = boss.y - py;
                            let d = ddx * ddx + ddy * ddy;
                            if d < 600.0 * 600.0 {
                                boss.hp -= dmg as f32;
                                self.bullets.spawn_effect(boss.x, boss.y, 0.10, 9);
                                self.particles.emit(boss.x, boss.y, 5, [0.3, 0.8, 1.0, 1.0]);
                                se.enemy_hit = true;
                            }
                        }
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
            // Step 20: 画面外判定をマップサイズ基準に変更（ワールド座標で判定）
            if bx < -100.0 || bx > MAP_WIDTH + 100.0 || by < -100.0 || by > MAP_HEIGHT + 100.0 {
                self.bullets.kill(i);
            }
        }

        // Bullet vs enemy collision（EnemyKind ごとの半径・経験値を使用）
        let bullet_query_r = BULLET_RADIUS + 32.0_f32;
        for bi in 0..bl {
            if !self.bullets.alive[bi] { continue; }
            let dmg = self.bullets.damage[bi];
            // ダメージ 0 はエフェクト専用弾（Whip / Lightning）— 衝突判定をスキップ
            if dmg == 0 { continue; }
            let bx       = self.bullets.positions_x[bi];
            let by       = self.bullets.positions_y[bi];
            let piercing = self.bullets.piercing[bi];
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
                        let prev_pending = self.level_up_pending;
                        self.check_level_up();
                        if self.level_up_pending && !prev_pending { se.level_up = true; }
                        // 撃破: タイプ別パーティクル
                        let pc = match kind {
                            EnemyKind::Slime => [1.0, 0.5, 0.1, 1.0],
                            EnemyKind::Bat   => [0.7, 0.2, 0.9, 1.0],
                            EnemyKind::Golem => [0.6, 0.6, 0.6, 1.0],
                        };
                        self.particles.emit(ex, ey, 8, pc);
                        se.enemy_death = true;
                        // Step 19: アイテムドロップ（1体につき最大1種類）
                        let roll = self.rng.next_u32() % 100;
                        let (item_kind, item_value) = if roll < 2 {
                            (ItemKind::Magnet, 0)
                        } else if roll < 7 {
                            (ItemKind::Potion, 20)
                        } else {
                            (ItemKind::Gem, kind.exp_reward())
                        };
                        self.items.spawn(ex, ey, item_kind, item_value);
                    } else {
                        // ヒット: 通常は黄色、Fireball は炎色パーティクル
                        let hit_color = if piercing { [1.0, 0.4, 0.0, 1.0] } else { [1.0, 0.9, 0.3, 1.0] };
                        self.particles.emit(ex, ey, 3, hit_color);
                        se.enemy_hit = true;
                    }
                    // 貫通弾は消えない
                    if !piercing {
                        self.bullets.kill(bi);
                        break;
                    }
                }
            }
        }

        // Step 19: アイテム更新（磁石エフェクト + 自動収集）
        {
            if self.magnet_timer > 0.0 {
                self.magnet_timer = (self.magnet_timer - dt).max(0.0);
            }
            // 磁石エフェクト: 宝石がプレイヤーに向かって飛んでくる
            if self.magnet_timer > 0.0 {
                let item_len = self.items.len();
                for i in 0..item_len {
                    if !self.items.alive[i] { continue; }
                    if self.items.kinds[i] != ItemKind::Gem { continue; }
                    let dx = px - self.items.positions_x[i];
                    let dy = py - self.items.positions_y[i];
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                    self.items.positions_x[i] += (dx / dist) * 300.0 * dt;
                    self.items.positions_y[i] += (dy / dist) * 300.0 * dt;
                }
            }
            // 自動収集判定
            let collect_r = if self.magnet_timer > 0.0 { 9999.0_f32 } else { 60.0_f32 };
            let collect_r_sq = collect_r * collect_r;
            let item_len = self.items.len();
            for i in 0..item_len {
                if !self.items.alive[i] { continue; }
                let dx = px - self.items.positions_x[i];
                let dy = py - self.items.positions_y[i];
                if dx * dx + dy * dy <= collect_r_sq {
                    match self.items.kinds[i] {
                        ItemKind::Gem => {}
                        ItemKind::Potion => {
                            self.player.hp = (self.player.hp + self.items.value[i] as f32)
                                .min(self.player.max_hp);
                            self.particles.emit(px, py, 6, [0.2, 1.0, 0.4, 1.0]);
                        }
                        ItemKind::Magnet => {
                            self.magnet_timer = 10.0;
                            self.particles.emit(px, py, 8, [1.0, 0.9, 0.2, 1.0]);
                        }
                    }
                    se.item_pickup = true;
                    self.items.kill(i);
                }
            }
        }

        // Step 24: ボス出現チェック（3 分 / 6 分 / 9 分）
        self.boss_spawned = false;
        {
            const BOSS_TIMES: [f32; 3] = [180.0, 360.0, 540.0];
            const BOSS_KINDS: [BossKind; 3] = [BossKind::SlimeKing, BossKind::BatLord, BossKind::StoneGolem];
            if self.boss.is_none() && self.next_boss_index < BOSS_TIMES.len() {
                if self.elapsed_seconds >= BOSS_TIMES[self.next_boss_index] {
                    let kind = BOSS_KINDS[self.next_boss_index];
                    // プレイヤーの画面外（右側）からスポーン
                    let bx = px + 600.0;
                    let by = py;
                    self.boss = Some(BossState::new(kind, bx, by));
                    self.next_boss_index += 1;
                    self.boss_spawned = true;
                    se.boss_spawn = true;
                }
            }
        }

        // Step 24: ボス更新（借用競合を避けるため、特殊行動データを先に取り出す）
        #[derive(Default)]
        struct BossAction {
            spawn_slimes:    bool,
            spawn_rocks:     bool,
            bat_dash_effect: bool,
            special_x:       f32,
            special_y:       f32,
            hurt_particle:   bool,
            hurt_x:          f32,
            hurt_y:          f32,
        }
        let mut boss_action = BossAction::default();

        if let Some(ref mut boss) = self.boss {
            if boss.alive {
                // 無敵タイマー更新
                if boss.invincible_timer > 0.0 {
                    boss.invincible_timer = (boss.invincible_timer - dt).max(0.0);
                    if boss.invincible_timer <= 0.0 {
                        boss.invincible = false;
                    }
                }

                // ボス AI: 種別ごとの移動
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

                // マップ内に制限
                boss.x = boss.x.clamp(boss.kind.radius(), MAP_WIDTH  - boss.kind.radius());
                boss.y = boss.y.clamp(boss.kind.radius(), MAP_HEIGHT - boss.kind.radius());

                // 特殊行動タイマー
                boss.phase_timer -= dt;
                if boss.phase_timer <= 0.0 {
                    boss.phase_timer = boss.kind.special_interval();
                    match boss.kind {
                        BossKind::SlimeKing => {
                            boss_action.spawn_slimes = true;
                            boss_action.special_x = boss.x;
                            boss_action.special_y = boss.y;
                        }
                        BossKind::BatLord => {
                            let ddx = px - boss.x;
                            let ddy = py - boss.y;
                            let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.001);
                            let dash_speed = 500.0;
                            boss.dash_vx = (ddx / dist) * dash_speed;
                            boss.dash_vy = (ddy / dist) * dash_speed;
                            boss.is_dashing = true;
                            boss.dash_timer = 0.6;
                            boss.invincible = true;
                            boss.invincible_timer = 0.6;
                            boss_action.bat_dash_effect = true;
                            boss_action.special_x = boss.x;
                            boss_action.special_y = boss.y;
                        }
                        BossKind::StoneGolem => {
                            boss_action.spawn_rocks = true;
                            boss_action.special_x = boss.x;
                            boss_action.special_y = boss.y;
                        }
                    }
                }

                // ボス vs プレイヤー接触ダメージ
                let boss_r = boss.kind.radius();
                let hit_r  = PLAYER_RADIUS + boss_r;
                let ddx = px - boss.x;
                let ddy = py - boss.y;
                if ddx * ddx + ddy * ddy < hit_r * hit_r {
                    if self.player.invincible_timer <= 0.0 && self.player.hp > 0.0 {
                        self.player.hp = (self.player.hp - boss.kind.damage_per_sec() * dt).max(0.0);
                        self.player.invincible_timer = INVINCIBLE_DURATION;
                        se.player_hurt = true;
                        boss_action.hurt_particle = true;
                        boss_action.hurt_x = px;
                        boss_action.hurt_y = py;
                    }
                }
            }
        }

        // 特殊行動の副作用（借用競合を避けるため boss 借用の外で実行）
        if boss_action.spawn_slimes {
            let positions: Vec<(f32, f32)> = (0..8).map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                (boss_action.special_x + angle.cos() * 120.0, boss_action.special_y + angle.sin() * 120.0)
            }).collect();
            self.enemies.spawn(&positions, EnemyKind::Slime);
            self.particles.emit(boss_action.special_x, boss_action.special_y, 16, [0.2, 1.0, 0.2, 1.0]);
        }
        if boss_action.spawn_rocks {
            let dirs: [(f32, f32); 4] = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)];
            for (dx_dir, dy_dir) in dirs {
                self.bullets.spawn_ex(
                    boss_action.special_x, boss_action.special_y,
                    dx_dir * 200.0, dy_dir * 200.0,
                    50, false, BULLET_KIND_ROCK,
                );
            }
            self.particles.emit(boss_action.special_x, boss_action.special_y, 10, [0.6, 0.6, 0.6, 1.0]);
        }
        if boss_action.bat_dash_effect {
            self.particles.emit(boss_action.special_x, boss_action.special_y, 12, [0.8, 0.2, 1.0, 1.0]);
        }
        if boss_action.hurt_particle {
            self.particles.emit(boss_action.hurt_x, boss_action.hurt_y, 8, [1.0, 0.15, 0.15, 1.0]);
        }

        // Step 24: 弾丸 vs ボス衝突判定
        let mut boss_killed = false;
        if let Some(ref mut boss) = self.boss {
            if boss.alive && !boss.invincible {
                let boss_r = boss.kind.radius();
                for bi in 0..self.bullets.len() {
                    if !self.bullets.alive[bi] { continue; }
                    let dmg = self.bullets.damage[bi];
                    if dmg == 0 { continue; }
                    let bx = self.bullets.positions_x[bi];
                    let by = self.bullets.positions_y[bi];
                    let hit_r = BULLET_RADIUS + boss_r;
                    let ddx = bx - boss.x;
                    let ddy = by - boss.y;
                    if ddx * ddx + ddy * ddy < hit_r * hit_r {
                        boss.hp -= dmg as f32;
                        se.enemy_hit = true;
                        if !self.bullets.piercing[bi] {
                            self.bullets.kill(bi);
                        }
                        if boss.hp <= 0.0 {
                            boss.alive = false;
                            boss_killed = true;
                            break;
                        }
                    }
                }
            }
        }
        if boss_killed {
            if let Some(ref boss) = self.boss {
                let exp = boss.kind.exp_reward();
                let bx = boss.x;
                let by = boss.y;
                self.score += exp * 2;
                self.exp   += exp;
                let prev_pending = self.level_up_pending;
                self.check_level_up();
                if self.level_up_pending && !prev_pending { se.level_up = true; }
                self.particles.emit(bx, by, 40, [1.0, 0.5, 0.0, 1.0]);
                se.enemy_death = true;
                for _ in 0..10 {
                    let ox = (self.rng.next_f32() - 0.5) * 200.0;
                    let oy = (self.rng.next_f32() - 0.5) * 200.0;
                    self.items.spawn(bx + ox, by + oy, ItemKind::Gem, exp / 10);
                }
            }
            self.boss = None;
        }

        // Step 24: Whip vs ボス
        // (Whip の処理は weapon_slots ループ内で行うため、ここでは別途チェック)
        // ※ Whip ダメージはすでに enemies に対して処理済みのため、ボス専用処理を追加
        // → Whip ダメージをボスに適用するため、weapon_slots ループ後にチェック
        // （実装上、Whip の当たり判定はループ内で行われているため、ここでは不要）

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

        se
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
                ("whip",       WeaponKind::Whip),
                ("fireball",   WeaponKind::Fireball),
                ("lightning",  WeaponKind::Lightning),
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
                "axe"       => WeaponKind::Axe,
                "cross"     => WeaponKind::Cross,
                "whip"      => WeaponKind::Whip,
                "fireball"  => WeaponKind::Fireball,
                "lightning" => WeaponKind::Lightning,
                _           => WeaponKind::MagicWand,
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

    /// Step 20: カメラオフセットを返す
    fn camera_offset(&self) -> (f32, f32) {
        (self.camera_x, self.camera_y)
    }

    /// Step 23/24: (x, y, kind, anim_frame) を返す
    fn get_render_data(&self) -> Vec<(f32, f32, u8, u8)> {
        let mut v = Vec::with_capacity(2 + self.enemies.len() + self.bullets.len());
        v.push((self.player.x, self.player.y, 0u8, self.player.anim_frame));
        // Step 24: ボスを描画（中心座標から左上に変換）
        if let Some(ref boss) = self.boss {
            if boss.alive {
                let boss_sprite_size = match boss.kind {
                    BossKind::StoneGolem => 128.0,
                    _ => 96.0,
                };
                v.push((
                    boss.x - boss_sprite_size / 2.0,
                    boss.y - boss_sprite_size / 2.0,
                    boss.kind.render_kind(),
                    0u8,
                ));
            }
        }
        for i in 0..self.enemies.len() {
            if self.enemies.alive[i] {
                v.push((
                    self.enemies.positions_x[i],
                    self.enemies.positions_y[i],
                    self.enemies.kinds[i].render_kind(),
                    self.enemies.anim_frames[i],
                ));
            }
        }
        for i in 0..self.bullets.len() {
            if self.bullets.alive[i] {
                v.push((self.bullets.positions_x[i], self.bullets.positions_y[i], self.bullets.render_kind[i], 0u8));
            }
        }
        v
    }

    fn get_item_data(&self) -> Vec<(f32, f32, u8)> {
        let mut v = Vec::with_capacity(self.items.count);
        for i in 0..self.items.len() {
            if self.items.alive[i] {
                v.push((
                    self.items.positions_x[i],
                    self.items.positions_y[i],
                    self.items.kinds[i].render_kind(),
                ));
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
            // Step 19: アイテム情報
            magnet_timer:    self.magnet_timer,
            item_count:      self.items.count,
            // Step 20: カメラ座標
            camera_x:        self.camera_x,
            camera_y:        self.camera_y,
            // Step 24: ボス情報
            boss_info:       self.boss.as_ref().filter(|b| b.alive).map(|b| BossHudInfo {
                name:   b.kind.name().to_string(),
                hp:     b.hp,
                max_hp: b.max_hp,
            }),
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

/// Step 20: マップ全体の外周からスポーン（カメラ位置に関係なくマップ端から出現）
fn spawn_outside(rng: &mut SimpleRng) -> (f32, f32) {
    let margin = 80.0;
    match rng.next_u32() % 4 {
        0 => (rng.next_f32() * MAP_WIDTH, -margin),
        1 => (rng.next_f32() * MAP_WIDTH, MAP_HEIGHT + margin),
        2 => (-margin,                     rng.next_f32() * MAP_HEIGHT),
        _ => (MAP_WIDTH + margin,          rng.next_f32() * MAP_HEIGHT),
    }
}

// ─── winit application ────────────────────────────────────────

struct App {
    window:      Option<Arc<Window>>,
    renderer:    Option<Renderer>,
    game:        GameWorld,
    keys_held:   HashSet<KeyCode>,
    last_update: Option<Instant>,
    // Step 22: 音声マネージャ（デバイスなし環境では None）
    audio:       Option<AudioManager>,
}

impl App {
    fn new() -> Self {
        Self {
            window:      None,
            renderer:    None,
            game:        GameWorld::new(),
            keys_held:   HashSet::new(),
            last_update: None,
            audio:       None,
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

        // Step 22: 音声デバイスを初期化し BGM を開始する
        let audio = AudioManager::new();
        if let Some(ref am) = audio {
            am.play_bgm(BGM_BYTES);
        }

        self.window      = Some(window);
        self.renderer    = Some(renderer);
        self.last_update = Some(Instant::now());
        self.audio       = audio;
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
                // Step 20: ゲーム側にも画面サイズを通知してカメラ計算を正確に保つ
                self.game.on_resize(size.width, size.height);
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
                    let se = self.game.step(dt);

                    // Step 22: SE 再生（音声デバイスが存在する場合のみ）
                    if let Some(ref am) = self.audio {
                        // レベルアップは最優先（他の SE より目立たせる）
                        if se.level_up {
                            am.play_se(LEVEL_UP_BYTES);
                        } else if se.enemy_death {
                            am.play_se(DEATH_BYTES);
                        } else if se.enemy_hit {
                            am.play_se(HIT_BYTES);
                        }
                        if se.player_hurt {
                            am.play_se(PLAYER_HURT_BYTES);
                        }
                        if se.item_pickup {
                            am.play_se_with_volume(ITEM_PICKUP_BYTES, 0.6);
                        }
                    }
                }
                self.last_update = Some(now);

                // ─── 描画 ──────────────────────────────────────
                if let (Some(renderer), Some(window)) =
                    (self.renderer.as_mut(), self.window.as_ref())
                {
                    let render_data    = self.game.get_render_data();   // Step 23: (x,y,kind,anim_frame)
                    let particle_data  = self.game.get_particle_data();
                    let item_data      = self.game.get_item_data();
                    let camera_offset  = self.game.camera_offset();
                    renderer.update_instances(&render_data, &particle_data, &item_data, camera_offset);
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
