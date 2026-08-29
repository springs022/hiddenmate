# AGENTS.md

このファイルは、AIエージェント（LLM）がHiddenMateを理解し、安全に開発・修正・検証するためのプロジェクトガイドです。手元の記憶より、リポジトリ内のコード・設定・設計文書を優先してください。

## 1. プロジェクト概要

**プロジェクト名**: HiddenMate
**目的**: 覆面駒を使用した協力詰・協力自玉詰を検討するソフトウェア。高速な協力詰エンジン[fmrs](https://github.com/ogiekako/fmrs)を基盤とする。

現在の実装状況:

- 覆面駒（Variable）を使用した協力詰・協力自玉詰に対応する。
- 覆面駒の候補世界、観測着手、候補絞り込み、全候補世界での詰み証明を実装している。
- 公開Web版は静的なGitHub Pagesで配信し、探索をWeb Worker内のWebAssemblyで実行する。問題や局面を外部サーバーへ送信しない。
- 同じ問題JSONと探索コアを使用するCLIを提供する。
- 透明駒（Invisible）の検討機能は開発中で、現時点では利用できない。

## 2. 構成と責務

### Rust

- **`rust/hiddenmate_core/`**: HiddenMate固有の中心実装。
  - 問題JSONの読み込み
  - 覆面駒の候補世界列挙
  - 観測着手と候補世界の更新
  - 協力詰・協力自玉詰探索
  - 日本語手順表記
- **`rust/hiddenmate_cli/`**: HiddenMate問題JSONを解くCLI。
- **`rust/wasm/`**: Web版から呼び出すWasmバインディング。`solve_variable_problem`がHiddenMate探索の境界となる。
- **`rust/fmrs_core/`**: fmrs由来の局面表現、SFEN、合法手生成、王手・詰み判定など。HiddenMateの各具体世界はこの通常局面実装を利用する。
- **`rust/src/`**: fmrs由来のCLI・探索・自動生成機能。HiddenMate固有の変更対象とは限らないため、依頼と関係がある場合だけ変更する。

### Web

- **`app/src/ui/component/VariableSolver.tsx`**: 覆面駒問題の入力、局面編集、ルール選択、探索結果表示を担う主要UI。
- **`app/src/solve/variable_solver_client.ts`**: UIと探索用Web Workerの間を仲介し、実行・中断・エラー処理を管理する。
- **`app/src/solve/variable_solver.worker.ts`**: Worker内でWasmの`solve_variable_problem`を実行する。
- **`app/src/wasm_api.ts` / `app/src/wasm_api.d.ts`**: TypeScriptとWasmのインターフェース。
- **`app/public/`**: README画像、OGP画像などの公開静的資産。
- **`docs/`**: Webpack/Wasmの生成物。原則としてソースを修正し、生成コマンドで更新する。

### 仕様・運用文書

- **`README.md`**: 利用者向けの概要、Web版、CLI、現在の実装状況。
- **`design/architecture.md`**: 候補世界、観測着手、探索、Web/CLI構成。
- **`design/problem-format.md`**: 問題JSONのフィールド、ルール、手番、候補駒種。
- **`DEPLOYMENT.md`**: 環境差、検証、本番反映、CI・公開確認のランブック。

## 3. HiddenMateのドメイン要点

- **覆面駒（Variable）**: 正体が未確定の駒。覆面駒自体を`fmrs_core`の新しい駒種にはせず、正体を具体化した複数の通常局面を候補世界として保持する。
- **候補世界**: これまでの観測と矛盾しない具体局面。合法性、駒の在庫、王手義務、着手の観測によって絞り込む。
- **観測着手**: 棋譜から見える情報へ具体着手を射影したもの。未確定の覆面駒では駒種を直接表示しない。
- **詰みの証明**: 残るすべての候補世界で、選択したルールの詰み条件が成立した場合だけ詰みとする。
- **対応ルール**:
  - `helpmate`: 協力詰。攻方が受方玉を詰める。
  - `helpSelfmate`: 協力自玉詰。受方が攻方玉を詰める。
- **持駒の覆面駒**:
  - `indistinguishable`: 同じ駒台の覆面駒を区別せず、どの個体だったかを後続の観測から推論する。既定値。
  - `distinguishable`: V1、V2のように個体を指定して着手する。
- **探索手数**: 指定手数以下について、ルール上詰みになり得る手数だけを短い順に列挙する。
- **制限**: 覆面駒は最大6枚。詳細は`design/problem-format.md`を正とする。

## 4. 作業開始時の必須確認

新しいチャット／タスクでは、実装前に次を確認する。

1. `git status --short`、現在のブランチ、`git remote -v`を確認し、ユーザーの未コミット変更を保護する。
2. `package.json`、`rust-toolchain.toml`、`.github/workflows/gh-pages.yaml`を確認し、実際のバージョンとCIコマンドを把握する。
3. `DEPLOYMENT.md`を読み、環境差、検証方法、本番反映条件を確認する。
4. Node.js、pnpm、Cargo、wasm-packの実体とバージョンを確認する。PATHにない場合はバンドル済みランタイムやユーザーディレクトリ配下も探す。
5. 変更対象の実装、テスト、設計文書を読み、既存仕様を推測だけで変更しない。

未コミット変更はユーザーの作業として扱い、明示的な依頼なしに破棄・上書き・混在させない。

## 5. 開発・検証コマンド

### HiddenMate CLI

プロジェクトルートから:

```console
cd rust
cargo run -p hiddenmate_cli --locked -- ../examples/variable-help-selfmate.json
```

### Rust / Wasm

ワーキングディレクトリは`rust/`。

```console
cargo test --workspace --no-fail-fast --locked
cargo clippy --all-targets --all-features --locked
cargo fmt --all --check
```

HiddenMateコアだけを素早く確認する場合:

```console
cargo test -p hiddenmate_core --locked
```

### Web

ワーキングディレクトリはプロジェクトルート。パッケージマネージャーとバージョンは`package.json`の`packageManager`を正とする。

```console
pnpm install --frozen-lockfile
pnpm test
pnpm build
```

開発サーバー:

```console
pnpm serve
```

本番モードのローカルプレビュー:

```console
pnpm serve-prod
```

確認URLは`http://localhost:3000/hiddenmate/`。

## 6. 変更範囲ごとの検証

- **`hiddenmate_core`のルール・候補世界・探索**: 関連テストを追加し、`cargo test -p hiddenmate_core --locked`とRustワークスペース全体を実行する。
- **`fmrs_core`の局面・合法手・詰み判定**: 関連テストを追加し、Rustワークスペース全体、Clippy、formatを実行する。性能への影響も確認する。
- **Wasm境界**: Rust/Wasmテストに加え、Webテストと本番ビルドを実行する。
- **React・TypeScript・CSS**: 関連テスト、全Webテスト、型検査を含む本番ビルドを実行する。
- **UI操作**: 自動テストに加え、必要に応じて本番モードのローカルプレビューで実操作する。
- **文書のみ**: 実装と設定を照合し、リンク、コマンド、サンプル、用語、`git diff --check`を確認する。記載した実行例は可能な範囲で実行して検証する。

本番反映を依頼された場合は、ローカル検証だけで完了とせず、`DEPLOYMENT.md`に従って対象ファイルだけをcommit・pushし、GitHub Actionsの全必須ジョブと公開サイト上の反映まで確認する。pushや公開は、ユーザーが明示的に依頼した場合だけ行う。

## 7. コーディング・変更方針

- **HiddenMate中心**: HiddenMate固有の機能は原則として`hiddenmate_core`、`hiddenmate_cli`、Wasm境界、覆面駒UIに実装し、fmrs由来コードへの不要な変更を避ける。
- **パフォーマンス**: 候補世界数に比例して処理量・メモリ使用量が増える。探索ループ内の不要な割り当て、クローン、全走査を避ける。`fmrs_core`の変更は通常探索全体への性能影響を特に慎重に確認する。
- **型安全性**: RustとTypeScriptの型を活用し、`any`の乱用や根拠のない`unwrap()`を避ける。
- **テスト**: バグ修正・ルール変更・探索変更では、再現テストまたは仕様テストを追加する。WebとRustで同じ問題形式を扱う変更では、両境界の整合性を確認する。
- **互換性**: 問題JSONの既定値や旧形式の読み込みを不用意に壊さない。形式変更時は`design/problem-format.md`、CLI、Wasm、UI、テストを同時に更新する。
- **生成物**: 生成済みWasmや`docs/`を手編集しない。ソースとビルド手順を正とする。
- **言語**: ドキュメント、コメント、コミットメッセージ、ユーザーとのやり取りは原則として日本語を使用する。
- **スコープ**: 依頼と無関係なfmrs既存機能、一本道詰将棋の自動生成、重い探索実験には変更を広げない。
