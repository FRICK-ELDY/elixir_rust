//! Path: native/game_native/src/lib.rs
//! Summary: NIF エントリ・ワールド型・物理ステップ・イベント・セーブをすべて含む game_native ライブラリ

// ベンチマーク等から利用するため re-export（後方互換）
pub use game_core::enemy::EnemyKind;
pub use game_core::boss::BossKind;


// ─── 1.5.5: デバッグ支援（NIF）────────────────────────────────
/// デバッグビルド時のみ: NIF パニック時に Rust のバックトレースを stderr に出力する。
/// RUST_BACKTRACE=1 でより詳細なバックトレースが得られる。
#[cfg(debug_assertions)]
fn init_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("[Rust NIF Panic] {}", info);
        eprintln!("Backtrace:\n{}", std::backtrace::Backtrace::force_capture());
    }));
}

// 1.5.1: GameLoop 制御用（pause/resume）
pub struct GameLoopControl {
    paused: std::sync::atomic::AtomicBool,
}

impl GameLoopControl {
    pub fn new() -> Self {
        Self {
            paused: std::sync::atomic::AtomicBool::new(false),
        }
    }
    pub fn pause(&self) {
        self.paused.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn resume(&self) {
        self.paused.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn is_paused(&self) -> bool {
        self.paused.load(std::sync::atomic::Ordering::SeqCst)
    }
}

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
    // 1.2.9: ボス種別アトム
    slime_king,
    bat_lord,
    stone_golem,
    // ゲーム状態アトム
    alive,
    dead,
    none,
    // 1.3.1: イベントバス用アトム
    enemy_killed,
    player_damaged,
    level_up_event,
    item_pickup,
    boss_defeated,
    // 1.5.1: Rust ゲームループ → Elixir 送信用
    frame_events,
}

// 1.4.7: 敵は u8 ID で参照。atom から ID への変換は Elixir の entity_registry で行う。

mod game_logic;
mod nif;
mod world;
pub use game_logic::{
    find_nearest_enemy, find_nearest_enemy_excluding, find_nearest_enemy_spatial,
    find_nearest_enemy_spatial_excluding, update_chase_ai, update_chase_ai_simd,
};
pub use world::{
    BossState, BulletWorld, EnemyWorld, FrameEvent, GameWorld, GameWorldInner, ParticleWorld, PlayerState,
    BULLET_KIND_FIREBALL, BULLET_KIND_LIGHTNING, BULLET_KIND_NORMAL, BULLET_KIND_ROCK, BULLET_KIND_WHIP,
};
pub use nif::{SaveSnapshot, WeaponSlotSave};

// ─── ローダー ─────────────────────────────────────────────────

#[allow(non_local_definitions)]
fn load(env: rustler::Env, _: rustler::Term) -> bool {
    // 1.5.5: デバッグビルド時のみパニックフックを設定（NIF クラッシュ時にバックトレース表示）
    #[cfg(debug_assertions)]
    init_panic_hook();
    // 1.5.5: RUST_LOG で Rust 側ログを有効化（例: RUST_LOG=debug）
    let _ = env_logger::Builder::from_default_env().try_init();

    let _ = rustler::resource!(GameWorld, env);
    let _ = rustler::resource!(GameLoopControl, env);
    // アトムを NIF ロード時に事前登録して、比較が確実に動作するようにする
    let _ = ok();
    let _ = slime();
    let _ = bat();
    let _ = golem();
    let _ = frame_events();
    true
}

rustler::init!("Elixir.App.NifBridge", load = load);

