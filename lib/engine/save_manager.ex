defmodule Engine.SaveManager do
  require Logger

  @moduledoc """
  Step 43: セーブ・ロード管理。

  ゲーム状態の永続化（セッション中断・再開）とハイスコアの保存を行う。
  Rust の get_save_snapshot / load_save_snapshot を呼び出し、
  Elixir 側で term_to_binary / File によりファイルに永続化する。
  """

  @session_path "saves/session.dat"
  @high_scores_path "saves/high_scores.dat"
  @high_scores_max 10

  @doc """
  現在のゲーム状態をセーブする。

  world_ref からスナップショットを取得し、saves/session.dat に保存する。

  ## 例

      Engine.SaveManager.save_session(world_ref)

  ## 戻り値

  - `:ok` - 保存成功
  - `{:error, reason}` - 保存失敗
  """
  def save_session(world_ref) do
    try do
      snapshot = App.NifBridge.get_save_snapshot(world_ref)
      binary = :erlang.term_to_binary(snapshot)
      File.mkdir_p!("saves")
      File.write!(@session_path, binary)
      :ok
    rescue
      e -> {:error, Exception.message(e)}
    end
  end

  @doc """
  セーブデータをロードして world_ref に復元する。

  ## 例

      case Engine.SaveManager.load_session(world_ref) do
        :ok -> # 復元成功
        :no_save -> # セーブデータなし
        {:error, reason} -> # ロード失敗（ファイル破損など）
      end

  ## 戻り値

  - `:ok` - 復元成功
  - `:no_save` - セーブファイルが存在しない
  - `{:error, reason}` - ロード失敗（ファイル破損など）
  """
  def load_session(world_ref) do
    case File.read(@session_path) do
      {:ok, binary} ->
        try do
          snapshot = :erlang.binary_to_term(binary)
          App.NifBridge.load_save_snapshot(world_ref, snapshot)
          :ok
        rescue
          e -> {:error, Exception.message(e)}
        end

      {:error, :enoent} ->
        :no_save

      {:error, reason} ->
        {:error, :file.format_error(reason)}
    end
  end

  @doc """
  セーブファイルが存在するかどうかを返す。

  ## 例

      if Engine.SaveManager.has_save?() do
        # 「ロード」ボタンを表示
      end
  """
  def has_save? do
    File.exists?(@session_path)
  end

  @doc """
  スコアをハイスコアとして記録する。

  上位 @high_scores_max 件を保持し、降順でソートして保存する。

  ## 例

      Engine.SaveManager.save_high_score(1500)

  ## 戻り値

  - `:ok` - 保存成功
  - `{:error, reason}` - 保存失敗
  """
  def save_high_score(score) when is_integer(score) and score >= 0 do
    try do
      current = load_high_scores()
      new_list = [score | current] |> Enum.uniq() |> Enum.sort(:desc) |> Enum.take(@high_scores_max)
      File.mkdir_p!("saves")
      File.write!(@high_scores_path, :erlang.term_to_binary(new_list))
      :ok
    rescue
      e -> {:error, Exception.message(e)}
    end
  end

  def save_high_score(_), do: {:error, :invalid_score}

  @doc """
  保存されているハイスコア一覧を取得する。

  上位から降順のリストで返す。保存データがなければ空リスト。

  ## 例

      scores = Engine.SaveManager.load_high_scores()
      # => [2000, 1500, 1200, ...]

  ## 戻り値

  - `[score, ...]` - ハイスコアのリスト（最大 @high_scores_max 件）
  """
  def load_high_scores do
    case File.read(@high_scores_path) do
      {:ok, binary} ->
        try do
          :erlang.binary_to_term(binary)
        rescue
          e ->
            Logger.warning("Failed to deserialize high scores: #{Exception.message(e)}")
            []
        end

      {:error, :enoent} ->
        []

      {:error, reason} ->
        Logger.warning("Failed to read high scores file: #{:file.format_error(reason)}")
        []
    end
  end

  @doc """
  ベストスコア（1位）を取得する。なければ nil。
  """
  def best_score do
    case load_high_scores() do
      [best | _] -> best
      [] -> nil
    end
  end
end
