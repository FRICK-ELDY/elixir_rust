# Elixir 環境セットアップガイド

**対象**: Windows  
**プロジェクト要件** (mix.exs より):

| ツール | バージョン |
|--------|------------|
| Elixir | ~> 1.19 |
| Erlang/OTP | 26.0 以上 |
| rustler | ~> 0.34 |

---

## 方法 1: Windows インストーラー（推奨・手軽）

1. **Erlang/OTP を先にインストール**
   - https://www.erlang.org/downloads から OTP 26/27/28 の Windows インストーラーをダウンロード
   - または [Erlang Solutions](https://www.erlang-solutions.com/resources/download.html)

2. **Elixir をインストール**
   - 使っている OTP バージョンに合わせて以下からダウンロード:
   - [Elixir 1.19.5 on Erlang 26](https://github.com/elixir-lang/elixir/releases/download/v1.19.5/elixir-otp-26.exe)
   - [Elixir 1.19.5 on Erlang 27](https://github.com/elixir-lang/elixir/releases/download/v1.19.5/elixir-otp-27.exe)
   - [Elixir 1.19.5 on Erlang 28](https://github.com/elixir-lang/elixir/releases/download/v1.19.5/elixir-otp-28.exe)

3. **OTP バージョンの確認**
   ```powershell
   erl -eval "erlang:display(erlang:system_info(otp_release)), halt()." -noshell
   ```

4. **確認**
   ```powershell
   elixir --version
   mix --version
   ```

---

## 方法 2: Chocolatey

```powershell
# 管理者権限の PowerShell で実行
choco install elixir
```

Elixir が Erlang を依存関係として自動インストールします。

---

## 方法 3: Scoop

```powershell
# Scoop が未インストールの場合
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
irm get.scoop.sh | iex

# Erlang と Elixir をインストール
scoop install erlang
scoop install elixir
```

---

## 方法 4: install.bat スクリプト（バージョン固定）

公式の PowerShell 用インストールスクリプトを使用:

```powershell
# ダウンロード
Invoke-WebRequest -Uri "https://elixir-lang.org/install.bat" -OutFile "install.bat"

# Elixir 1.19.5 + OTP 28.1 をインストール
.\install.bat elixir@1.19.5 otp@28.1

# PATH に追加（同じターミナルセッションで即反映）
$installs_dir = "$env:USERPROFILE\.elixir-install\installs"
$env:PATH = "$installs_dir\otp\28.1\bin;$env:PATH"
$env:PATH = "$installs_dir\elixir\1.19.5-otp-28\bin;$env:PATH"

# 確認
iex.bat
```

永続的に PATH を設定する場合は、システムの環境変数設定から上記のパスを追加してください。

---

## PATH 設定（インストール後もコマンドが見つからない場合）

Windows インストーラーは PATH に自動追加しない場合があります。以下の手順で設定してください。

### Git Bash (MINGW64) を使っている場合

Git Bash は Windows の環境変数をそのまま参照しないことがあります。以下を試してください。

**その場で有効にする（現在のターミナルセッションのみ）:**
```bash
export PATH="/c/Program Files/Erlang OTP/bin:/c/Program Files/Elixir/bin:$PATH"
elixir --version
```

**恒久設定** — `~/.bashrc` に追加:
```bash
echo 'export PATH="/c/Program Files/Erlang OTP/bin:/c/Program Files/Elixir/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

> **補足**: Git Bash では `C:\` を `/c/` と表記します。PowerShell やコマンドプロンプトでは問題なく使える場合、Git Bash 固有の設定として上記を追加してください。

---

### Cursor / VS Code の PowerShell で PATH が反映されない場合

環境変数には登録済みなのに、Cursor のターミナルでは `elixir` が見つからない場合があります。Cursor が古い環境を継承しているためです。

**即効（現在のターミナルのみ）:**
```powershell
$env:PATH = "C:\Program Files\Erlang OTP\bin;C:\Program Files\Elixir\bin;" + $env:PATH
elixir --version
```

**恒久対策 — PowerShell プロファイルに追加:**
```powershell
# プロファイルがなければ作成
if (!(Test-Path $PROFILE)) { New-Item -Path $PROFILE -ItemType File -Force }

# Elixir/Erlang の PATH を追記
Add-Content $PROFILE "`n`$env:PATH = `"C:\Program Files\Erlang OTP\bin;C:\Program Files\Elixir\bin;`" + `$env:PATH"

# 反映
. $PROFILE
elixir --version
```

これで、Cursor 内の PowerShell を開くたびに自動で PATH が追加されます。

---

### 即効確認（PATH が反映されていない場合）

環境変数は**起動中のアプリに伝わりません**。以下を試してください:

1. **Cursor を完全に終了**（ウィンドウを閉じるだけでなく、タスクトレイからも終了）
2. **Cursor を起動し直す**
3. 新しいターミナルで `elixir --version` を実行

まだダメな場合、環境変数画面で **Erlang のパスが Elixir より上（先）** になっているか確認してください。

### デフォルトのインストール先（参考）

| ソフトウェア | 想定パス |
|-------------|----------|
| Erlang OTP | `C:\Program Files\Erlang OTP\bin` または `C:\Program Files\erl-28.3.2\bin` |
| Elixir (elixir-otp-28.exe) | `C:\Program Files\Elixir\bin` または `C:\Program Files (x86)\Elixir\bin` |

> **重要**: **Erlang を Elixir より先に** PATH に追加してください。Elixir のスクリプトは内部で `erl.exe` を呼び出すため、Erlang が PATH に含まれていないと `elixir` や `mix` が動作しません。

### 1. インストール場所を確認する

PowerShell で以下を実行し、ファイルの場所を探します:

```powershell
# Erlang (erl.exe) の場所を検索
Get-ChildItem -Path "C:\Program Files" -Filter "erl.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object FullName

# Elixir (elixir.bat) の場所を検索
Get-ChildItem -Path "C:\Program Files*" -Filter "elixir.bat" -Recurse -ErrorAction SilentlyContinue | Select-Object FullName
```

見つかったディレクトリの **親フォルダの `bin`** を PATH に追加します。

例:
- `C:\Program Files\erl-28.3.2\bin\erl.exe` → PATH に `C:\Program Files\erl-28.3.2\bin` を追加
- `C:\Program Files (x86)\Elixir\bin\elixir.bat` → PATH に `C:\Program Files (x86)\Elixir\bin` を追加

### 2. PATH を設定する方法

#### 方法 A: システムの環境変数から設定（永続的・推奨）

1. `Win + R` → `sysdm.cpl` 入力 → Enter
2. **詳細設定** タブ → **環境変数**
3. **ユーザー環境変数**（またはシステム環境変数）の **Path** を選択 → **編集**
4. **新規** で次のパスを追加（**Erlang を先に**）:
   ```
   C:\Program Files\Erlang OTP\bin
   C:\Program Files\Elixir\bin
   ```
5. **OK** で閉じる
6. **Cursor を含む全てのターミナルを閉じ、Cursor を再起動**してから新しいターミナルで確認

#### 方法 B: PowerShell で永続的に追加

```powershell
# 現在のユーザーに PATH を永続追加
$erlangPath = "C:\Program Files\erl-28.3.2\bin"   # 実際のパスに合わせて変更
$elixirPath = "C:\Program Files (x86)\Elixir\bin" # 実際のパスに合わせて変更

$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
$newPath = "$erlangPath;$elixirPath;$currentPath"
[Environment]::SetEnvironmentVariable("Path", $newPath, "User")

# ターミナルを閉じて開き直してから確認
```

#### 方法 C: 現在のセッションのみ（一時的）

```powershell
$env:PATH = "C:\Program Files\erl-28.3.2\bin;C:\Program Files (x86)\Elixir\bin;$env:PATH"
elixir --version
mix --version
```

### 3. 動作確認

**新しいターミナル**を開いて実行:

```powershell
erl -eval "io:format(\"OTP ~s~n\", [erlang:system_info(otp_release)]), halt()." -noshell
elixir --version
mix --version
```

### PowerShell 実行ポリシー（mix が「デジタル署名されていません」で失敗する場合）

`mix` は内部的に `mix.ps1` を実行します。PowerShell のデフォルト設定では未署名スクリプトがブロックされます。

**対処（CurrentUser に限定し、ローカルスクリプトを許可）:**
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

実行後、`mix --version` を再実行してください。

> **補足**: `RemoteSigned` はローカルスクリプトは実行可能、インターネットから取得したスクリプトは署名が必要という設定です。開発環境では一般的な設定です。

---

## インストール後の確認

```powershell
cd D:\Work\FRICK-ELDY\elixir_rust

# 依存関係取得
mix deps.get

# コンパイル（Rust クレートもビルドされる）
mix compile

# 起動
mix run --no-halt
# または
iex -S mix
```

---

## トラブルシューティング

| 現象 | 対処 |
|------|------|
| `mix` や `elixir` が見つからない | PATH に Elixir の bin を追加。**Cursor を再起動**してターミナルを開き直す。 |
| `"erl.exe" は認識されません` と Elixir 実行時に出る | **Erlang の bin を PATH に追加**し、**Elixir より前に**配置する。 |
| PATH を設定したのに反映されない | 環境変数は起動中のアプリには伝わらない。**Cursor を完全に終了してから起動し直す**。 |
| `mix.ps1 はデジタル署名されていません` / 実行ポリシーエラー | 下記「PowerShell 実行ポリシー」を参照。 |
| `rustler` のコンパイルエラー | Rust がインストールされているか確認: `cargo --version` |
| OTP バージョン不一致 | Elixir 1.19 は OTP 26 以上が必要。`erl` で OTP を確認。 |
