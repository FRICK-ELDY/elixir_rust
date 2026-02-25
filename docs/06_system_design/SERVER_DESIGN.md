# サーバー設計（Phoenix Channels 導入を視野に）

**友達とつなぐの第一選択**: 自前サーバーの維持コストを避けるため、**まずは [Epic Online Services（EOS）](./EPIC_ONLINE_SERVICES.md) を採用**する。**EOS と自前サーバーは設定で切り替え可能**とする方針（[EPIC_ONLINE_SERVICES.md §1.3](./EPIC_ONLINE_SERVICES.md#13-eos-と自前サーバーの切り替え)）。本ドキュメントは、自前サーバーを選んだ場合の Phoenix 設計、または EOS と併用するゲームルーム専用サーバーの指針として参照する。

**根拠**: [STEPS_EXTENSION.md § Step 44](../05_steps/01_engine/STEPS_EXTENSION.md#6-step-44-マルチプレイ)、[MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md)

Phoenix Channels で「友達とつなぐ」サーバーを導入する際の、サーバー全体設計の指針。ゲームルーム（RoomChannel）の手前にある、認証・フレンド・メッセージ・プレゼンスなどのレイヤーを扱う。

---

## 1. 位置づけ

### 1.1 本ドキュメントと他ドキュメントの関係

| ドキュメント | 役割 |
|-------------|------|
| **本ドキュメント（SERVER_DESIGN.md）** | サーバー全体像：認証、フレンド、メッセージ、プレゼンス、通知。Phoenix のどの機能で実現するか。 |
| [MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md) | ゲームルームに特化：RoomChannel、入力配信、状態同期、Engine 連携。 |

ゲームルームへの参加フローは「認証 → プレゼンス/フレンド → ルーム参加」の順で考える。本ドキュメントはその前半（サーバー・ソーシャル層）を定義する。

### 1.2 サーバーの役割

- **友達とつなぐための Phoenix サーバー**を立て、クライアントは WebSocket（Channels）で接続する。
- 同一 Phoenix アプリ内で、Engine（RoomSupervisor / GameEvents）が動く。ルーム参加時は既存の [RoomChannel 設計](./MULTIPLAYER_PHOENIX_CHANNELS.md) に従う。
- フレンド機能・メッセージ・プレゼンスは Elixir/Phoenix の本領が発揮される領域として、サーバー設計に含める。

---

## 2. 機能レイヤーと Phoenix での実現

| 領域 | 想定する機能 | 実現手段（Phoenix/Elixir） |
|------|--------------|----------------------------|
| **認証・識別** | ログイン、セッション、ユーザー ID | Phoenix 認証（Auth）、Socket の `assigns`（`user_id` 等） |
| **プレゼンス** | 誰がオンラインか、どのルームにいるか | [Phoenix.Presence](https://hexdocs.pm/phoenix/Phoenix.Presence.html)（Channel に組み込み） |
| **フレンド・ソーシャル** | フレンド申請/承認、フレンドリスト、オンライン状態 | Ecto でリレーション保存、Presence でオンライン取得、Channels で通知 |
| **メッセージ** | テキスト/通知、既読、履歴 | 専用 Channel（例: `user:123`）または topic、Ecto で永続化、Channels で push |
| **通知** | フレンド申請、ルーム招待、メッセージ着信 | PubSub → ユーザー用 Channel に push、または Presence と組み合わせ |
| **マッチング・ルーム** | ルーム作成/参加/退出、招待 | [MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md) の RoomChannel + Engine.start_room / stop_room |

---

## 3. 認証・識別

- クライアントが Socket 接続する時点で **ユーザーを一意に識別** する必要がある。
- **推奨**: Phoenix の認証（[phx.gen.auth](https://hexdocs.pm/phoenix/authentication.html) 等）でユーザー・セッションを管理し、`connect/3` で `user_id` を `socket.assigns` に載せる。
- 全 Channel で `socket.assigns.user_id` を前提にできるため、フレンド・メッセージ・ルーム参加の権限チェックがしやすい。
- **メモリ**: 同時接続数が多いゲームサーバーでは、`user` 構造体全体を assigns に持たせるとメモリが増大する。`user_id` のみ保持し、必要なときだけ DB から取得するか、表示に必要な最小限のフィールドのみ保持する設計を推奨する。

```elixir
# イメージ: connect で user_id のみ assigns に（メモリ節約）
def connect(params, socket, _connect_info) do
  case verify_user(params) do
    {:ok, user} -> {:ok, assign(socket, :user_id, user.id)}
    _ -> :error
  end
end
```

---

## 4. プレゼンス

- **誰がオンラインか**・**どのルームにいるか**をリアルタイムで共有する。
- **Phoenix.Presence** を利用する。`topic` を「グローバル」または「ルームごと」に分ける設計が考えられる。

| 方式 | topic 例 | 用途 |
|------|-----------|------|
| グローバル | `"lobby"` や `"presence:global"` | オンラインユーザー一覧、フレンドのオンライン状態 |
| ルーム単位 | `"room:" <> room_id` | そのルームにいるメンバー（既存 RoomChannel と同居または連携） |

- フレンド機能では「フレンドのオンライン/オフライン」を Presence で追い、必要に応じて `user:123` のような topic に通知を push する。

---

## 5. フレンド機能

- **フレンド申請**・**承認/拒否**・**フレンドリスト**をサーバーで管理する。
- **永続化**: Ecto で `friendships`（または `friend_requests` / `friends`）テーブルを用意し、`user_id` / `friend_id` / `state`（pending, accepted）などを保存する。
- **リアルタイム**: 申請・承認時に、対象ユーザーの Channel（例: `"user:" <> user_id`）に `push socket, "friend_request"` や `"friend_accept"` を送る。
- **オンライン状態**: Presence の `"lobby"`（またはグローバル topic）で、フレンドの `user_id` が list にいるかどうかでオンライン判定する。

実装の優先度を分ける場合の例:

1. **Phase 1**: フレンド申請・承認・リスト表示（HTTP API または Channel の `handle_in`）
2. **Phase 2**: 申請/承認のリアルタイム通知（Channel push）
3. **Phase 3**: フレンドのオンライン状態表示（Presence 連携）

---

## 6. メッセージ機能

- **テキストメッセージ**・**既読**・**履歴**を想定する。
- **永続化**: Ecto で `messages` テーブル（送信者、受信者、本文、既読フラグ、タイムスタンプ等）を管理する。
- **配送**: 送信時に、受信者のトピック（例: `"user:" <> to_user_id`）へ PubSub で broadcast し、受信側の Channel プロセスがそれを受け取ってクライアントへ `push` する。未接続の場合は DB に残し、次回接続時に履歴 API または Channel の `join` 返却で渡す。
- **履歴取得**: 初回 join 時や専用 `handle_in("list_messages", ...)` で、Ecto から取得して返す。

---

## 7. 通知

- **フレンド申請**・**ルーム招待**・**メッセージ着信**などをクライアントに即時伝える。
- **方式**: 各イベント発生時に、対象ユーザーの Channel（`"user:" <> user_id`）に `push socket, "notification", %{type: "friend_request", ...}` のように送る。
- PubSub を使う場合: サーバー側で `Phoenix.PubSub.broadcast(MyApp.PubSub, "user:" <> user_id, {:notification, payload})` し、そのユーザーを subscribe している Channel が `handle_info({:notification, payload}, socket)` で受け取り、`push` する。

---

## 8. Channel 構成の整理

| topic 例 | 役割 | 参照 |
|----------|------|------|
| `"user:" <> user_id` | そのユーザー向けのメッセージ・通知・フレンドイベント | 本ドキュメント §5–7 |
| `"room:" <> room_id` | ゲームルーム。参加・入力・状態同期 | [MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md) |
| `"lobby"` または `"presence:global"` | プレゼンス（オンライン一覧） | 本ドキュメント §4 |

- クライアントは接続後、少なくとも `"user:" <> my_user_id` に join し、必要に応じて `"room:" <> room_id` に join してゲームを行う。

---

## 9. 多人数時の通信方式・負荷分散（方針）

プレイヤーが増えると「全員のゲームデータをサーバーが中継する」構成ではサーバー負荷が高くなる。以下の方針を採用する。

### 9.1 人数に応じた方式の使い分け

| 人数の目安 | 方式 | サーバー（Phoenix）の役割 | ゲームの「中心」 |
|------------|------|---------------------------|------------------|
| **少人数（2〜4 人）** | WebRTC P2P（メッシュ） | マッチング・シグナリングのみ。ゲームの高頻度データはサーバーを流さない。 | 各クライアントが対等に P2P で通信 |
| **多人数** | **ホスト 1 人 + 他はホストに P2P**、または **専用ゲームサーバー** | マッチングだけ / またはサーバーがゲームの中心 | ホストのクライアント / またはサーバー上の Engine |

- **メッシュの人数について**: フルメッシュでは各クライアントが（人数−1）本の接続を維持し、アップロード帯域・CPU・接続維持の負荷がかかる。一般的な家庭用ネットワークでは 8 人規模は厳しく不安定になりやすいため、メッシュの推奨は **3〜4 人程度**までとし、それ以上はスター型（ホスト・クライアント）や SFU への切り替えを検討する。

### 9.2 多人数時の二択（理想方針）

多人数では次のいずれかを採用することを方針とする。

| 方式 | 説明 | サーバーの役割 | このプロジェクトでの対応 |
|------|------|-----------------|---------------------------|
| **ホスト 1 人 + 他は P2P** | 1 プレイヤーをホストとし、他プレイヤーはホストのクライアントに接続。ゲームのループ・判定はホスト側で行う。 | マッチング・ルーム一覧・「誰がホストか」の通知、NAT トラバーサル補助。ゲームデータは流さない。 | Engine（GameEvents + GameWorld）をホストのクライアントで動かす。Phoenix はルーム作成・参加・WebRTC シグナリングのみ。 |
| **専用ゲームサーバー** | 1 台のサーバーがゲームの権威となり、全プレイヤーがそのサーバーに接続する。 | マッチングに加え、サーバー上で Engine を動かし、入力の集約・状態の配信を行う。 | [MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md) の想定どおり、Phoenix アプリ内で RoomSupervisor / GameEvents を動かす。全員が RoomChannel 経由で接続。 |

- **ホスト方式**: サーバー負荷を抑えられる。ホストの回線・マシンがボトルネックになる。友達同士・少〜中規模向け。
- **専用ゲームサーバー方式**: 公平性・チート耐性・安定性を重視する場合に適する。サーバー運用コストは増える。
- **ホスト方式と NAT**: ホスト（Listen Server）は家庭用ネットワークの NAT 内側にいることが多く、他プレイヤーがホストに接続するには **STUN/TURN サーバーによる NAT トラバーサル（穴あけ）** がほぼ必須になる。専用サーバー方式ではサーバーが公網側にあるためこの問題が小さい。ホスト方式を採用する場合は STUN/TURN の導入・運用を実装コストとして見込む。

### 9.3 補足

- 少人数で WebRTC を使う場合、サーバーは「誰がルームにいるか」「WebRTC の offer/answer・ICE の受け渡し」だけを担当する。
- 多人数で「入力だけサーバーで配る」中間案（入力ブロードキャスト）もあり得るが、本方針では多人数時は**ホスト or 専用ゲームサーバー**のいずれかを採用するものとする。

---

## 10. 実装の優先順位（提案）

| 順序 | 項目 | 内容 |
|------|------|------|
| 1 | 認証 | Socket で `user_id` を識別できるようにする |
| 2 | プレゼンス（最小） | グローバルまたは lobby でオンライン一覧を表示できるようにする |
| 3 | ゲームルーム連携 | [MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md) に従い RoomChannel を実装する（Step 44b） |
| 4 | フレンド（基本） | 申請・承認・リストの API または Channel |
| 5 | 通知 | フレンド申請・承認を `user:` Channel で push |
| 6 | メッセージ | テキスト送受信・履歴・既読（必要に応じて） |

---

## 11. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [EPIC_ONLINE_SERVICES.md](./EPIC_ONLINE_SERVICES.md) | EOS 採用方針（友達とつなぐの第一選択）。マッチング・ロビー・フレンド・ボイスを EOS で賄う設計 |
| [MULTIPLAYER_PHOENIX_CHANNELS.md](./MULTIPLAYER_PHOENIX_CHANNELS.md) | ゲームルーム（RoomChannel）・Engine 連携・入力・状態同期 |
| [STEPS_EXTENSION.md § Step 44](../05_steps/01_engine/STEPS_EXTENSION.md#6-step-44-マルチプレイ) | Step 44 の目標・44a/44b の区別 |
| [ENGINE_API.md](./ENGINE_API.md) | Engine の API（start_room, get_loop_for_room 等） |
| [ELIXIR_RUST_DIVISION.md § 4.2](../03_tech_decisions/ELIXIR_RUST_DIVISION.md) | 競技マルチプレイ・determinism の考え方 |
