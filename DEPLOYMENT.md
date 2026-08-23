# HiddenMateの公開方法

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

## 将来の重い探索

ブラウザのメモリ・実行時間を超える作品が必要になった段階で、既存fmrsと同様にCloud Run APIを追加する。

- 通常の短編：ブラウザWasm
- 大量候補世界・長編：Cloud Run
- CLI：ローカルPCの全メモリとCPUを利用

覆面駒MVPではGitHub PagesとCLIを優先し、サーバーは必須にしない。
