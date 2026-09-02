# HiddenMate 開発・本番反映ランブック

この文書は、新しいチャット／タスクでも環境依存の調査を繰り返さず、安全に実装から本番確認まで進めるための手順書です。実行コマンドやバージョンは将来変わり得るため、常にリポジトリ内の設定と実際のツール出力を最優先してください。

## 作業開始時の確認

プロジェクトルートで次を確認します。

```powershell
git status --short
git branch --show-current
git remote -v
Get-Content package.json
Get-Content rust-toolchain.toml
Get-Content .github/workflows/gh-pages.yaml
```

- 通常の作業ブランチは`main`、公開先リポジトリは`origin`（`springs022/hiddenmate`）です。ただし、コマンド実行時の出力を正とします。
- 未コミットの変更はユーザーの作業として扱い、上書き・破棄しません。
- push、公開、外部への書き込みは、ユーザーが明示的に依頼した場合だけ行います。「本番反映まで」はcommit、`main`へのpush、CI、公開サイト確認までを含みます。

## ツールと環境差

### Web

- CIはNode.js 22と、`package.json`で指定されたpnpmを使用します。
- CodexのWindows環境では、Node.jsがインストール済みでも`node`や`pnpm`が`PATH`にない場合があります。
- まず`Get-Command node,pnpm -ErrorAction SilentlyContinue`で確認します。見つからない場合はCodexのワークスペース依存ランタイムを取得し、返されたNode.jsの絶対パスでスクリプトを直接実行します。
- CIとの差を疑う不具合では、Node.js 22でも再現を確認します。一時ランタイムを使う場合は、公式配布物とチェックサムを用い、作業後にワークスペース内の一時ディレクトリだけを削除します。

pnpmが利用できる通常環境では、次を使用します。

```powershell
pnpm install --frozen-lockfile
pnpm test
pnpm build
```

pnpmが`PATH`にない一方、依存関係が既に導入済みなら、Node.jsの絶対パスを`<NODE>`として次のように個別実行できます。

```powershell
& <NODE> node_modules/vitest/vitest.mjs run
& <NODE> node_modules/typescript/bin/tsc -p app/tsconfig.json --noEmit
& <NODE> node_modules/webpack/bin/webpack.js --mode production
```

### Rust / Wasm

- Rustのチャンネルは`rust-toolchain.toml`、CIの正確なコマンドは`.github/workflows/gh-pages.yaml`を正とします。
- Windowsで`cargo`が`PATH`にない場合は、`Get-Command cargo -ErrorAction SilentlyContinue`に加えて、ユーザーディレクトリ配下の`.cargo/bin/cargo.exe`を確認します。ユーザー名を含む絶対パスは手順書に固定しません。
- ローカルにClippyコンポーネントがなければ、その事実を記録し、最終的にはCIの`Rust Clippy`で確認します。
- Windowsでは、Unix前提の`/dev/null`を使う既存テストが権限エラーで失敗することがあります。変更起因かを個別テストとLinux CIで切り分け、単に成功扱いにはしません。

基本コマンドは次のとおりです。

```powershell
Set-Location rust
cargo test --workspace --no-fail-fast --locked
cargo clippy --all-targets --all-features --locked
cargo fmt --all --check
Set-Location ..
```

## 変更範囲ごとの検証

- React・TypeScript・CSSのみ: 関連テスト、全Webテスト、TypeScript型検査、本番ビルド。
- Rustコア: 関連テスト、Rustワークスペース全体のテスト、Clippy、format。探索処理は性能劣化にも注意します。
- Wasm境界: Rust/Wasmテストに加えWebテストと本番ビルド。
- UI操作: 自動テストに加え、必要に応じて開発サーバーまたは公開サイトで実操作を確認します。

### VitestとWasmの既知の注意点

Webテストから`app/src/model/index.ts`などのbarrel exportを経由すると、テスト対象に不要なWasm ES moduleまで読み込まれ、特にLinux CIで失敗することがあります。テストでは必要な値を定義元のモジュールから直接importしてください。

例:

```ts
import { emptyBoard } from "../../model/board";
```

ローカルで通っても、CIのNode.js 22/Linuxでのみ発生する可能性があります。本番反映時は必ずCI結果まで確認します。

## 推奨構成

初期公開はGitHub Pagesだけを使用する。

```text
GitHub mainブランチ
  └─ GitHub Actions
       ├─ Rust/Wasmビルド
       ├─ Reactビルド
       └─ gh-pagesブランチへ配置

利用者のブラウザ
  └─ Wasmで局面生成・探索を実行
```

静的サイトなのでサーバー運用費がかからず、問題局面も外部へ送信しない。リポジトリ名を`hiddenmate`として公開すると、標準URLは次の形になる。

```text
https://<GitHubユーザー名>.github.io/hiddenmate/
```

`.github/workflows/gh-pages.yaml`が`main`へのpushを契機にビルド・公開する。GitHub側ではRepository SettingsのPagesで、`gh-pages`ブランチを公開元に設定する。

## 本番反映

公開フローは`.github/workflows/gh-pages.yaml`で定義されています。`main`へのpushを契機に次が実行され、すべて成功した場合だけ`gh-pages`へ配備されます。

1. `Rust Test`
2. `Rust Clippy`
3. `Web Test and Build`
4. `Deploy GitHub Pages`

手順:

1. `git diff`と`git status --short`で変更範囲を確認する。
2. 必要なローカル検証を完了する。
3. 日本語のコミットメッセージでcommitし、`main`を`origin`へpushする。
4. 対象commit SHAに対応するGitHub Actions runを特定する。
5. 上記4ジョブがすべて`success`になるまで確認する。
6. `https://springs022.github.io/hiddenmate/`を開き、HTTP応答だけでなく、変更した機能または生成物が実際に配信されていることを確認する。

## 一時プレビュー

本番トップページを変更せず、GitHub Pagesの`dev/`配下へ一時プレビューを置く場合は、
Webpackへ公開先のベースパスを明示する。

```powershell
pnpm exec webpack --mode production --env basePath=/hiddenmate/dev/<preview-name>/
```

生成された`docs/`の内容を`gh-pages`ブランチの`dev/<preview-name>/`へ配置する。
プレビューは公開URLとなるため、ユーザーの明示的な依頼がある場合だけpushし、確認後は同じディレクトリを削除する。

GitHub ActionsのステータスはGitHub UI、`gh run`、またはGitHub APIで確認できます。公開APIでジョブ詳細が取得できない場合、ログ閲覧にはGitHubへの認証が必要です。資格情報やトークンをコマンド出力・ログ・文書へ表示してはいけません。

## 公開サイトのキャッシュ確認

ブラウザやCDNのキャッシュで古い`main.js`が見える場合があります。公開HTMLから実際のアセット名を確認し、クエリ文字列を付けて再取得します。

```text
https://springs022.github.io/hiddenmate/main.js?v=<現在時刻など>
```

ファイル名がハッシュ化された場合は、上記の固定名を決め打ちせず公開HTMLに記載されたURLを使います。HTTP 200だけでは不十分で、必要なら変更固有の文字列や挙動も確認します。

## 完了報告に含めるもの

- 実装・修正した内容
- 実施したローカル検証と結果
- commit SHAとpush先（本番反映を依頼された場合）
- GitHub Actionsの結果とrun URL
- 公開URLと本番上で確認した内容
- 環境差による既知の失敗や、未実施の確認があればその理由

ローカルテスト成功、push成功、CI成功、公開確認はそれぞれ別の状態です。本番反映の依頼では、最後の公開確認まで終えてから完了とします。

## 将来の重い探索

ブラウザのメモリ・実行時間を超える作品が必要になった段階で、既存fmrsと同様にCloud Run APIを追加する。

- 通常の短編：ブラウザWasm
- 大量候補世界・長編：Cloud Run
- CLI：ローカルPCの全メモリとCPUを利用

覆面駒MVPではGitHub PagesとCLIを優先し、サーバーは必須にしない。
