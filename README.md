# HiddenMate

覆面駒（Variable）と透明駒（Invisible）を扱う、フェアリー協力詰の検討・創作支援ソフトウェアです。

高速な通常協力詰エンジン [fmrs](https://github.com/ogiekako/fmrs) を基盤にしています。確定した通常局面の合法手生成は `fmrs_core` に任せ、HiddenMate は「ここまでの手順と矛盾しない具体局面の集合」を管理します。

## 現在の実装状況

最初の覆面駒MVPを実装中です。現在は次に対応しています。

- 通常将棋14駒種を候補とする、位置・所属既知の覆面駒
- 複数候補世界の列挙
- 標準駒数からの受方持駒補完
- 初形の二歩、行き所、王数、王手状態による候補除外
- 攻方王手義務・受方応手による候補世界の絞り込み
- 覆面駒の移動、成、駒取り、持駒化、再打の個体追跡
- 残る全候補世界に対する証明済み詰み判定
- 指定手数の協力手順列挙
- JSON問題形式とCLI
- WebAssembly経由のWeb検討（JSON入力、候補世界・解答表示）

Web版は [HiddenMate](https://springs022.github.io/hiddenmate/) で試せます。入力済みのサンプル問題は「覆面駒を検討」ボタンだけで実行できます。Web盤面上での覆面駒の直接配置と透明駒は今後実装します。

## CLI

Rust環境で次を実行します。

```console
cd rust
cargo run -p hiddenmate_cli -- ../examples/variable-rook-dragon.json
```

出力例：

```text
初形候補世界: 2
  V1: {Rook, ProRook}
解数: 1
1: V1:64-84
```

問題形式は [design/problem-format.md](design/problem-format.md) を参照してください。

## 開発

```console
cd rust
cargo test --workspace --no-fail-fast
cargo clippy --all-targets --all-features
```

Web版：

```console
npm install
npm run serve
```

設計は [design/architecture.md](design/architecture.md)、公開方法は [DEPLOYMENT.md](DEPLOYMENT.md) に記載しています。

## ライセンスと由来

MIT Licenseです。基盤であるfmrsの著作権表示とライセンス条件を引き継ぎます。詳細は [LICENSE](LICENSE) と [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) を参照してください。
