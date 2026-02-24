# マルチプレイ・Phoenix Channels 連携

**根拠**: [STEPS_MAP_SAVE_MULTI_DEBUG.md § Step 44](../05_steps/STEPS_MAP_SAVE_MULTI_DEBUG.md#6-step-44-マルチプレイ)

Step 44 で用意したルーム管理基盤と Phoenix Channels を連携する際の設計指針。

---

## 1. 前提

- **Engine.RoomSupervisor**: ルーム ID ごとに GameLoop + GameWorld を起動
- **Engine.RoomRegistry**: room_id → GameLoop pid のマッピング
- 設計オプション A（複数 GameWorld）採用: 1 ルーム = 1 GameWorld、同一ルーム内のプレイヤー間衝突は将来対応

---

## 2. Phoenix Channel との連携フロー

### 2.1 ルーム参加

```elixir
# lib/your_app_web/channels/room_channel.ex（将来実装例）

def join("room:" <> room_id, _params, socket) do
  case Engine.start_room(room_id) do
    {:error, reason} when reason != :already_started ->
      {:error, %{reason: "room_start_failed"}}
    _ ->
      {:ok, assign(socket, :room_id, room_id)}
  end
end
```

### 2.2 入力イベントの配信

クライアントから入力（移動・攻撃等）を受信し、該当ルームの GameLoop に渡す。

```elixir
# 入力イベント受信時
def handle_in("input", %{"dx" => dx, "dy" => dy}, socket) do
  room_id = socket.assigns.room_id

  case Engine.get_loop_for_room(room_id) do
    {:ok, pid} ->
      # world_ref は GameLoop の state にあるため、
      # 入力は GameLoop に cast で渡し、内部で set_player_input を呼ぶ設計
      send(pid, {:remote_input, dx, dy})
      {:reply, :ok, socket}

    :error ->
      {:reply, {:error, %{reason: "room_not_found"}}, socket}
  end
end
```

**補足**: 現状の GameLoop はローカル InputHandler の ETS を参照している。リモート入力対応には、GameLoop に `{:remote_input, dx, dy}` の `handle_info` を追加し、そのルームの `world_ref` に対して `Engine.set_player_input` を呼ぶ実装が必要。

### 2.3 状態同期

| 方式 | 説明 |
|------|------|
| **入力ブロードキャスト** | 各クライアントの入力をサーバーで集約し、全クライアントにブロードキャスト。クライアント側で同じ physics を再現（determinism 前提） |
| **スナップショット配信** | サーバーが定期的に `get_frame_metadata` / `get_save_snapshot` 相当のデータを取得し、クライアントに push |

---

## 3. 実装チェックリスト（将来）

- [ ] Phoenix プロジェクトに `game` アプリを依存追加
- [ ] `RoomChannel` で `join("room:" <> id)` 時に `Engine.start_room(id)` を呼ぶ
- [ ] `leave` 時に `Engine.stop_room(id)`（最後のクライアントが退出したときのみ）
- [ ] GameLoop に `{:remote_input, dx, dy}` の `handle_info` を追加
- [ ] クライアントへの状態 push（`push socket, "state", payload`）の周期・形式を決定

---

## 4. 関連ドキュメント

| ドキュメント | 用途 |
|-------------|------|
| [STEPS_MAP_SAVE_MULTI_DEBUG.md § Step 44](../05_steps/STEPS_MAP_SAVE_MULTI_DEBUG.md#6-step-44-マルチプレイ) | Step 44 の目標・実装内容 |
| [ELIXIR_RUST_DIVISION.md § 4.2](../03_tech_decisions/ELIXIR_RUST_DIVISION.md) | 競技マルチプレイ（ロールバック等）の determinism について |
