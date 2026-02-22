/// Standalone rendering binary.
/// Runs the full game loop in pure Rust without Elixir/NIF.
/// Used for renderer development and visual testing.
mod constants;
mod renderer;
mod physics;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use constants::{
    BULLET_DAMAGE, BULLET_LIFETIME, BULLET_RADIUS, BULLET_SPEED,
    CELL_SIZE, ENEMY_DAMAGE_PER_SEC, ENEMY_RADIUS, ENEMY_SEPARATION_FORCE,
    ENEMY_SEPARATION_RADIUS, INVINCIBLE_DURATION,
    MAX_ENEMIES, PLAYER_RADIUS, PLAYER_SIZE, PLAYER_SPEED,
    SCREEN_HEIGHT, SCREEN_WIDTH, WAVES, WEAPON_COOLDOWN,
};
use renderer::{HudData, Renderer};
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
    count:        usize,
    /// 分離パス用の作業バッファ（毎フレーム再利用してアロケーションを回避）
    sep_x:        Vec<f32>,
    sep_y:        Vec<f32>,
}

impl EnemyWorld {
    fn new() -> Self {
        Self { positions_x: Vec::new(), positions_y: Vec::new(), hp: Vec::new(), alive: Vec::new(), count: 0, sep_x: Vec::new(), sep_y: Vec::new() }
    }
    fn spawn(&mut self, positions: &[(f32, f32)]) {
        for &(x, y) in positions {
            let slot = self.alive.iter().position(|&a| !a);
            if let Some(i) = slot {
                self.positions_x[i] = x;
                self.positions_y[i] = y;
                self.hp[i]    = 30.0;
                self.alive[i] = true;
            } else {
                self.positions_x.push(x);
                self.positions_y.push(y);
                self.hp.push(30.0);
                self.alive.push(true);
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
}

struct BulletWorld {
    positions_x:  Vec<f32>,
    positions_y:  Vec<f32>,
    velocities_x: Vec<f32>,
    velocities_y: Vec<f32>,
    lifetime:     Vec<f32>,
    alive:        Vec<bool>,
    count:        usize,
}

impl BulletWorld {
    fn new() -> Self {
        Self { positions_x: Vec::new(), positions_y: Vec::new(), velocities_x: Vec::new(), velocities_y: Vec::new(), lifetime: Vec::new(), alive: Vec::new(), count: 0 }
    }
    fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32) {
        let slot = self.alive.iter().position(|&a| !a);
        if let Some(i) = slot {
            self.positions_x[i]  = x;
            self.positions_y[i]  = y;
            self.velocities_x[i] = vx;
            self.velocities_y[i] = vy;
            self.lifetime[i]     = BULLET_LIFETIME;
            self.alive[i]        = true;
        } else {
            self.positions_x.push(x);
            self.positions_y.push(y);
            self.velocities_x.push(vx);
            self.velocities_y.push(vy);
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
    weapon_cooldown:  f32,
    exp:              u32,
    level:            u32,
    level_up_pending: bool,
    weapon_choices:   Vec<String>,
    level_up_timer:   f32,
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
            weapon_cooldown:  0.0,
            exp:              0,
            level:            1,
            level_up_pending: false,
            weapon_choices:   Vec::new(),
            level_up_timer:   0.0,
        }
    }

    fn step(&mut self, dt: f32) {
        if self.level_up_pending {
            self.level_up_timer += dt;
            if self.level_up_timer >= 3.0 {
                self.confirm_level_up();
            }
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

        // 敵 AI
        for i in 0..self.enemies.len() {
            if !self.enemies.alive[i] { continue; }
            let ex = self.enemies.positions_x[i];
            let ey = self.enemies.positions_y[i];
            let ddx = px - ex;
            let ddy = py - ey;
            let dist = (ddx * ddx + ddy * ddy).sqrt().max(0.001);
            self.enemies.positions_x[i] += (ddx / dist) * 80.0 * dt;
            self.enemies.positions_y[i] += (ddy / dist) * 80.0 * dt;
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

        // プレイヤー vs 敵
        let hit_r = PLAYER_RADIUS + ENEMY_RADIUS;
        let candidates = self.collision.dynamic.query_nearby(px, py, hit_r);
        for idx in candidates {
            if !self.enemies.alive[idx] { continue; }
            let ex = self.enemies.positions_x[idx] + ENEMY_RADIUS;
            let ey = self.enemies.positions_y[idx] + ENEMY_RADIUS;
            let ddx = px - ex;
            let ddy = py - ey;
            if ddx * ddx + ddy * ddy < hit_r * hit_r {
                if self.player.invincible_timer <= 0.0 && self.player.hp > 0.0 {
                    self.player.hp = (self.player.hp - ENEMY_DAMAGE_PER_SEC * dt).max(0.0);
                    self.player.invincible_timer = INVINCIBLE_DURATION;
                    // 赤いパーティクル
                    self.particles.emit(px, py, 6, [1.0, 0.15, 0.15, 1.0]);
                }
            }
        }

        // 武器（Magic Wand）
        self.weapon_cooldown = (self.weapon_cooldown - dt).max(0.0);
        if self.weapon_cooldown <= 0.0 {
            if let Some(ti) = self.find_nearest_enemy(px, py) {
                let tx  = self.enemies.positions_x[ti] + ENEMY_RADIUS;
                let ty  = self.enemies.positions_y[ti] + ENEMY_RADIUS;
                let bdx = tx - px;
                let bdy = ty - py;
                let bl  = (bdx * bdx + bdy * bdy).sqrt().max(0.001);
                self.bullets.spawn(px, py, (bdx / bl) * BULLET_SPEED, (bdy / bl) * BULLET_SPEED);
                self.weapon_cooldown = WEAPON_COOLDOWN;
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

        // Bullet vs enemy collision
        let hit_r2 = BULLET_RADIUS + ENEMY_RADIUS;
        for bi in 0..bl {
            if !self.bullets.alive[bi] { continue; }
            let bx = self.bullets.positions_x[bi];
            let by = self.bullets.positions_y[bi];
            let nearby = self.collision.dynamic.query_nearby(bx, by, hit_r2);
            for ei in nearby {
                if !self.enemies.alive[ei] { continue; }
                let ex  = self.enemies.positions_x[ei] + ENEMY_RADIUS;
                let ey  = self.enemies.positions_y[ei] + ENEMY_RADIUS;
                let ddx = bx - ex;
                let ddy = by - ey;
                if ddx * ddx + ddy * ddy < hit_r2 * hit_r2 {
                    self.enemies.hp[ei] -= BULLET_DAMAGE as f32;
                    if self.enemies.hp[ei] <= 0.0 {
                        self.enemies.kill(ei);
                        self.score += 10;
                        self.exp   += 5;
                        self.check_level_up();
                        // 撃破: オレンジパーティクル
                        self.particles.emit(ex, ey, 8, [1.0, 0.5, 0.1, 1.0]);
                    } else {
                        // ヒット: 黄色パーティクル
                        self.particles.emit(ex, ey, 3, [1.0, 0.9, 0.3, 1.0]);
                    }
                    self.bullets.kill(bi);
                    break;
                }
            }
        }

        // Wave-based enemy spawn
        let (wave_interval, wave_count) = current_wave(self.elapsed_seconds);
        if self.elapsed_seconds - self.last_spawn_secs >= wave_interval
            && self.enemies.count < MAX_ENEMIES
        {
            let to_spawn = wave_count.min(MAX_ENEMIES - self.enemies.count);
            let positions: Vec<(f32, f32)> = (0..to_spawn)
                .map(|_| spawn_outside(&mut self.rng))
                .collect();
            self.enemies.spawn(&positions);
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
            self.level_up_timer   = 0.0;
            self.weapon_choices   = vec![
                "magic_wand".to_string(),
                "axe".to_string(),
                "cross".to_string(),
            ];
        }
    }

    fn confirm_level_up(&mut self) {
        self.level            += 1;
        self.level_up_pending  = false;
        self.level_up_timer    = 0.0;
        self.weapon_choices.clear();
    }

    fn get_render_data(&self) -> Vec<(f32, f32, u8)> {
        let mut v = Vec::with_capacity(1 + self.enemies.len() + self.bullets.len());
        v.push((self.player.x, self.player.y, 0u8));
        for i in 0..self.enemies.len() {
            if self.enemies.alive[i] {
                v.push((self.enemies.positions_x[i], self.enemies.positions_y[i], 1u8));
            }
        }
        for i in 0..self.bullets.len() {
            if self.bullets.alive[i] {
                v.push((self.bullets.positions_x[i], self.bullets.positions_y[i], 2u8));
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
                    renderer.render(window, &hud);
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
