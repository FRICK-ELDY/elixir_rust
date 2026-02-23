# G3: アセット管理システム設計

**根拠**: [PRIORITY_STEPS.md](../04_roadmap/PRIORITY_STEPS.md) G3 — 実行時ロードがないため大規模ゲームにスケールしない

---

## 1. 責務分離

| レイヤー | 担当 | 具体例 |
|---------|------|--------|
| **Elixir** | 状態・ロード指示 | アセット ID、パスマッピングの決定、シーンに応じたロード優先度、非同期ロードのトリガー |
| **Rust** | バイナリ処理 | 画像・音声の読み込み、GPU テクスチャ化、キャッシュ、NIF 境界でバイナリを渡さない |

画像バイナリは Elixir を経由しない。Elixir は「どのアセットをいつロードするか」のポリシーのみを管理する。

---

## 2. アセット ID → パス マッピング

### 2.1 AssetId 列挙型

スプライトアトラス・音声を一意に識別する ID を定義する。

```rust
pub enum AssetId {
    SpriteAtlas,      // assets/sprites/atlas.png
    Bgm,
    HitSfx,
    DeathSfx,
    LevelUpSfx,
    PlayerHurtSfx,
    ItemPickupSfx,
}
```

### 2.2 デフォルトパス

| AssetId | デフォルトパス |
|---------|---------------|
| SpriteAtlas | `assets/sprites/atlas.png` |
| Bgm | `assets/audio/bgm.wav` |
| HitSfx | `assets/audio/hit.wav` |
| ... | ... |

### 2.3 パス解決

- **実行時ロード**: `base_path` + 相対パス でファイルを読み込む
- **埋め込みフォールバック**: ファイルが存在しない場合は `include_bytes!` の埋め込みデータを使用
- **base_path**: プロジェクトルートまたは実行ファイル基準。Elixir から文字列で渡すことも可能（将来）

---

## 3. 実装フェーズ

### Phase 1: スプライト差し替え（本実装）

1. **AssetId と AssetRegistry** の導入
2. **AssetLoader** による実行時ロード（ファイル存在時）＋埋め込みフォールバック
3. **Renderer** を `include_bytes!` 直接参照から AssetLoader 経由に変更
4. アセットベースパスを環境変数 `GAME_ASSETS_PATH` で指定可能にする

### Phase 2: 非同期ロード（将来）

- バックグラウンドスレッドで画像読み込み
- ロード完了コールバックまたはポーリング
- シーン遷移時に事前ロード

### Phase 3: キャッシュ・メモリ管理（将来）

- 未使用アセットの解放
- シーン別のキャッシュポリシー

---

## 4. ディレクトリ構造

```
assets/
├── sprites/               # デフォルト（game_id 未指定時）
│   ├── atlas.png          # メインスプライトアトラス
│   └── atlas_alt.png      # 差し替え用（将来的）
├── audio/
│   ├── bgm.wav
│   ├── hit.wav
│   └── ...
├── vampire_survivor/      # Step 39: ゲーム別アセット
│   ├── sprites/
│   │   └── atlas.png
│   └── audio/
│       └── ...
└── rhythm_game/           # 別ゲーム例
    ├── sprites/
    └── audio/
```

---

## 5. Step 39: ゲーム別アセットパス

ゲームごとに `assets/{game_id}/` をベースパスとして切り替え可能。

### 5.1 環境変数

| 変数 | 説明 |
|------|------|
| `GAME_ASSETS_PATH` | プロジェクトルート（任意） |
| `GAME_ASSETS_ID` | ゲーム ID（例: `vampire_survivor`）。指定時は `assets/{id}/` を優先参照 |

### 5.2 AssetLoader API

```rust
// 環境変数から自動読み込み（GAME_ASSETS_ID を参照）
let loader = AssetLoader::new();

// ゲーム ID を明示指定
let loader = AssetLoader::with_game_assets("vampire_survivor");
```

### 5.3 ロード順序

1. `assets/{game_id}/sprites/atlas.png`（GAME_ASSETS_ID 指定時）
2. `assets/sprites/atlas.png`（デフォルト）
3. 埋め込みフォールバック

### 5.4 Elixir 連携

- `Engine.Game` behaviour に `assets_path/0` コールバックを追加
- `config :game, current: Game.VampireSurvivor` のゲームが `assets_path/0` を返す
- Application 起動時に `GAME_ASSETS_ID` を設定
- `bin/start.bat` で未設定時は `vampire_survivor` をデフォルトに設定

---

## 6. 使用方法

### Rust 側（レンダラ）

```rust
// 従来
let atlas_bytes = include_bytes!("../../../../assets/sprites/atlas.png");

// 本設計後（Phase 1 実装済み）
let loader = crate::asset::AssetLoader::new();
let atlas_bytes = loader.load_sprite_atlas();
```

### スプライト差し替え（Phase 1）

環境変数 `GAME_ASSETS_PATH` にプロジェクトルートを指定すると、ファイルシステムからアトラスをロードする:

```powershell
# Windows（プロジェクトルートで実行する場合）
$env:GAME_ASSETS_PATH = (Get-Location).Path
cargo run --bin game_window
```

### ゲーム別アセット（Step 39）

```powershell
# ゲーム ID を指定して assets/vampire_survivor/ を参照
$env:GAME_ASSETS_ID = "vampire_survivor"
cargo run --bin game_window
```

`bin/start.bat` は未設定時に `GAME_ASSETS_ID=vampire_survivor` をデフォルトに設定する。

ファイルが存在しない場合は従来パス・埋め込みデータへフォールバックする。

### Elixir 側（将来拡張）

- `Game.NifBridge.set_assets_base_path(path)` でベースパスを設定
- アセット ID の列挙を共有するため、Elixir にアトムでマッピングを渡す設計も検討

---

## 7. 関連ドキュメント

- [PRIORITY_STEPS.md](../04_roadmap/PRIORITY_STEPS.md) — G3 設計方針
- [ENGINE_ANALYSIS.md](../02_spec_design/ENGINE_ANALYSIS.md) — 弱み分析（アセット埋め込み）
