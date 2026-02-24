# システムアーキテクチャ（全体像）

**根拠**: [SERVER_DESIGN.md](./SERVER_DESIGN.md)、[MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md)、[ENGINE_API.md](./ENGINE_API.md)

マルチプレイ・友達連携を視野に入れた、クライアント〜サーバー〜エンジンまでの全体アーキテクチャを定義する。

---

## 1. 全体構成

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ クライアント（ゲームクライアント / ブラウザ）                                 │
│   WebSocket → Phoenix Socket（認証済み user_id）                              │
│   join: "user:" <> user_id 必須 / "room:" <> room_id はゲーム参加時          │
└───────────────────────────────┬─────────────────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────────────────┐
│ Phoenix サーバー（友達とつなぐ・マッチング・通知）                           │
│                                                                             │
│ 認証・識別     Socket connect で user_id を assigns、全 Channel で利用       │
│ プレゼンス     Phoenix.Presence（lobby / room 単位）                         │
│ フレンド       Ecto 永続化 + user: Channel で申請・承認の push               │
│ メッセージ     Ecto 永続化 + user: Channel で配送、履歴 API                   │
│ 通知           user: Channel へ PubSub broadcast → push                     │
│                                                                             │
│ Channel 構成:                                                                │
│   "user:" <> user_id  … メッセージ・通知・フレンドイベント                    │
│   "room:" <> room_id  … ゲームルーム（入力・状態同期）→ Engine 連携          │
│   "lobby" / "presence:global" … プレゼンス（オンライン一覧）                  │
└───────────────────────────────┬─────────────────────────────────────────────┘
                                │ RoomChannel join → Engine.start_room
                                │ handle_in("input", ...) → GameLoop
┌───────────────────────────────▼─────────────────────────────────────────────┐
│ 同一 Phoenix アプリ内: Engine（Elixir + Rust NIF）                           │
│                                                                             │
│ RoomSupervisor / RoomRegistry  … ルーム ID ごとに GameLoop + GameWorld       │
│ GameLoop GenServer             … frame_events 受信・フェーズ管理・NIF 呼び出し │
│ Rust NIF (Rustler)             … ResourceArc<RwLock<GameWorld>>             │
│ Rust Native                    … ECS(SoA), Physics, wgpu Renderer           │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **前半（サーバー・ソーシャル層）**: [SERVER_DESIGN.md](./SERVER_DESIGN.md) で定義（認証 → プレゼンス/フレンド → ルーム参加）。
- **ゲームルーム層**: [MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md) の RoomChannel + Engine API。
- **エンジン層**: [ENGINE_API.md](./ENGINE_API.md)、[SPEC_ENGINE.md](../01_setup/SPEC_ENGINE.md)。

---

## 2. 接続・参加フロー

1. **認証**  
   クライアントが Socket 接続。`connect/3` でユーザーを検証し、`assign(socket, :user_id, user.id)` のみ保持（メモリ節約）。

2. **ユーザー Channel**  
   接続後、少なくとも `"user:" <> my_user_id` に join。メッセージ・通知・フレンドイベントを受信。

3. **プレゼンス**  
   必要に応じて `"lobby"`（または `"presence:global"`）に join し、オンライン一覧・フレンドのオンライン状態を表示。

4. **ゲームルーム参加**  
   ルーム作成・参加時に `"room:" <> room_id` に join。RoomChannel が `Engine.start_room(room_id)` を呼び、入力・状態同期は [MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md) に従う。

---

## 3. 機能レイヤーと責務

| レイヤー | 責務 | 参照 |
|----------|------|------|
| **認証・識別** | ログイン、セッション、Socket の `user_id` | SERVER_DESIGN §3 |
| **プレゼンス** | 誰がオンラインか、どのルームにいるか | SERVER_DESIGN §4 |
| **フレンド・ソーシャル** | 申請・承認・リスト、オンライン状態 | SERVER_DESIGN §5 |
| **メッセージ** | テキスト送受信、既読、履歴 | SERVER_DESIGN §6 |
| **通知** | フレンド申請・ルーム招待・メッセージ着信の push | SERVER_DESIGN §7 |
| **マッチング・ルーム** | ルーム作成/参加/退出、RoomChannel、Engine 連携 | MULTIPLAYER_PHOENIX_CHANNELS |
| **エンジン** | ルームごとの GameLoop / GameWorld、物理・描画 | ENGINE_API, SPEC_ENGINE |

---

## 4. 実装優先順位（SERVER_DESIGN 準拠）

| 順序 | 項目 | 内容 |
|------|------|------|
| 1 | 認証 | Socket で `user_id` を識別できるようにする |
| 2 | プレゼンス（最小） | グローバルまたは lobby でオンライン一覧を表示 |
| 3 | ゲームルーム連携 | RoomChannel を実装（Step 44b）、Engine.start_room / get_loop_for_room |
| 4 | フレンド（基本） | 申請・承認・リストの API または Channel |
| 5 | 通知 | フレンド申請・承認を `user:` Channel で push |
| 6 | メッセージ | テキスト送受信・履歴・既読（必要に応じて） |

---

## 5. 多人数時の通信方式（方針）

| 人数の目安 | 方式 | サーバーの役割 |
|------------|------|----------------|
| **少人数（2〜4 人）** | WebRTC P2P（メッシュ） | マッチング・シグナリングのみ |
| **多人数** | ホスト 1 人 + 他は P2P、または **専用ゲームサーバー** | マッチングのみ / またはサーバー上で Engine を実行 |

詳細は [SERVER_DESIGN.md §9](./SERVER_DESIGN.md#9-多人数時の通信方式負荷分散方針) を参照。

---

## 6. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [SERVER_DESIGN.md](./SERVER_DESIGN.md) | サーバー設計（認証・プレゼンス・フレンド・メッセージ・通知・Channel 構成） |
| [MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md) | ゲームルーム（RoomChannel）・Engine 連携・入力・状態同期 |
| [ENGINE_API.md](./ENGINE_API.md) | Engine API（start_room, get_loop_for_room 等） |
| [SPEC_ENGINE.md](../01_setup/SPEC_ENGINE.md) | エンジン層の技術アーキテクチャ・ECS・NIF |
