//! Path: native/game_native/src/nif/load.rs
//! Summary: NIF ローダー（パニックフック・リソース登録・アトム事前登録）

use crate::world::{GameLoopControl, GameWorld};

/// 1.5.5: デバッグビルド時のみ: NIF パニック時に Rust のバックトレースを stderr に出力する。
#[cfg(debug_assertions)]
fn init_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("[Rust NIF Panic] {}", info);
        eprintln!("Backtrace:\n{}", std::backtrace::Backtrace::force_capture());
    }));
}

#[allow(non_local_definitions)]
pub fn load(env: rustler::Env, _: rustler::Term) -> bool {
    #[cfg(debug_assertions)]
    init_panic_hook();
    let _ = env_logger::Builder::from_default_env().try_init();

    let _ = rustler::resource!(GameWorld, env);
    let _ = rustler::resource!(GameLoopControl, env);
    let _ = crate::ok();
    let _ = crate::slime();
    let _ = crate::bat();
    let _ = crate::golem();
    let _ = crate::frame_events();
    true
}
