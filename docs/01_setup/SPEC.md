# ゲーム仕様書（総合）

**プロジェクト名**: Elixir x Rust  
**プラットフォーム**: Windows / macOS / Linux（wgpu 対応環境）

仕様は **エンジン層** と **ゲーム層** に分離している。

---

## ドキュメント構成

| ドキュメント | 内容 |
|---|---|
| **[SPEC_ENGINE.md](SPEC_ENGINE.md)** | エンジン層：技術アーキテクチャ、ECS（SoA・EntityId）、Rustler NIF API、wgpu レンダリング、物理（Spatial Hash・衝突）、Elixir Supervisor／ゲームループ役割、パフォーマンス仕様 |
| **[SPEC_GAME_Survivor.md](SPEC_GAME_Survivor.md)** | ゲーム層：Survivor（ヴァンパイアサバイバーライク）。ゲームデザイン、プレイヤー／敵／武器／ステージ、Survivor 用 ECS コンポーネント、敵 AI、SpawnSystem、テクスチャアトラス |
| **[SPEC_GAME_mini_shooter.md](SPEC_GAME_mini_shooter.md)** | ゲーム層：Mini Shooter（トップダウンシューティング）。ゲームデザイン、プレイヤー／敵／弾／ウェーブ、Mini Shooter 用 ECS、敵 AI・スポーン、テクスチャアトラス |

---

## 参照の目安

- **エンジン・NIF・レンダリング・物理・性能** → [SPEC_ENGINE.md](SPEC_ENGINE.md)
- **Survivor のルール・データ・AI** → [SPEC_GAME_Survivor.md](SPEC_GAME_Survivor.md)
- **Mini Shooter のルール・データ・AI** → [SPEC_GAME_mini_shooter.md](SPEC_GAME_mini_shooter.md)
