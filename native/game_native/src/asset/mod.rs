//! G3: アセット管理システム
//!
//! アセット ID → パス のマッピング、実行時ロード、埋め込みフォールバックを提供。
//! 設計: [docs/ASSET_MANAGEMENT.md](https://github.com/...)

use std::path::Path;

/// アセット ID とパスの定義を1箇所に集約（single source of truth）
macro_rules! define_assets {
    ($($id:ident => $path:literal),* $(,)?) => {
        /// アセットを一意に識別する ID
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[allow(dead_code)] // Phase 2 で音声ロードに使用予定
        pub enum AssetId {
            $($id,)*
        }

        impl AssetId {
            /// デフォルトの相対パス（プロジェクトルート基準）
            pub fn default_path(&self) -> &'static str {
                match self {
                    $(AssetId::$id => $path,)*
                }
            }
        }

        fn load_asset_embedded(id: AssetId) -> Vec<u8> {
            match id {
                $(AssetId::$id => include_bytes!(concat!("../../../../", $path)).to_vec(),)*
            }
        }
    };
}

define_assets! {
    SpriteAtlas => "assets/sprites/atlas.png",
    Bgm => "assets/audio/bgm.wav",
    HitSfx => "assets/audio/hit.wav",
    DeathSfx => "assets/audio/death.wav",
    LevelUpSfx => "assets/audio/level_up.wav",
    PlayerHurtSfx => "assets/audio/player_hurt.wav",
    ItemPickupSfx => "assets/audio/item_pickup.wav",
}

/// アセットのロードを行う。実行時ロード（ファイル存在時）＋埋め込みフォールバック。
pub struct AssetLoader {
    /// ベースパス（プロジェクトルート相当）。None の場合はカレントディレクトリまたは埋め込みを使用
    base_path: Option<std::path::PathBuf>,
}

impl Default for AssetLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader {
    /// 環境変数 `GAME_ASSETS_PATH` が設定されていればベースパスとして使用する
    pub fn new() -> Self {
        let base_path = std::env::var("GAME_ASSETS_PATH")
            .ok()
            .map(std::path::PathBuf::from);
        Self { base_path }
    }

    /// カスタムベースパスを指定して作成（Elixir からのパス指定用・将来拡張）
    #[allow(dead_code)]
    pub fn with_base_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            base_path: Some(path.as_ref().to_path_buf()),
        }
    }

    /// アセットのバイト列をロードする。
    /// 1. base_path + 相対パス でファイルが存在すればそこから読み込む
    /// 2. カレントディレクトリからの相対パスで存在すれば読み込む
    /// 3. どちらも失敗すればコンパイル時埋め込みデータを使用
    pub fn load_bytes(&self, id: AssetId) -> Vec<u8> {
        let path = id.default_path();

        // 1. ベースパス + 相対パス
        if let Some(ref base) = self.base_path {
            if let Ok(bytes) = std::fs::read(base.join(path)) {
                return bytes;
            }
        }

        // 2. カレントディレクトリからの相対パス
        if let Ok(bytes) = std::fs::read(path) {
            return bytes;
        }

        // 3. 埋め込みフォールバック
        self.load_embedded(id)
    }

    /// スプライトアトラスのバイト列をロード（利便性のためのショートカット）
    pub fn load_sprite_atlas(&self) -> Vec<u8> {
        self.load_bytes(AssetId::SpriteAtlas)
    }

    /// 音声アセットのバイト列をロード（Phase 2 で AudioManager 連携予定）
    #[allow(dead_code)]
    pub fn load_audio(&self, id: AssetId) -> Vec<u8> {
        self.load_bytes(id)
    }

    /// コンパイル時埋め込みデータ（ファイルが存在しない場合のフォールバック）
    fn load_embedded(&self, id: AssetId) -> Vec<u8> {
        load_asset_embedded(id)
    }
}
