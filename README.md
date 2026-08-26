# HiddenMate

覆面駒（Variable）と透明駒（Invisible）を扱う、フェアリー協力詰の検討・創作支援ソフトウェアです。

高速な通常協力詰エンジン [fmrs](https://github.com/ogiekako/fmrs) を基盤にしています。確定した通常局面の合法手生成は `fmrs_core` に任せ、HiddenMate は「ここまでの手順と矛盾しない具体局面の集合」を管理します。

## 現在の実装状況

最初の覆面駒MVPを実装中です。現在は次に対応しています。

- 候補を常に通常将棋の全14駒種とする、位置・所属既知の覆面駒
- 盤上と攻方・受方駒台への覆面駒配置
- 複数候補世界の列挙
- 標準駒数からの受方持駒補完
- 初形の二歩、行き所、王数、王手状態による候補除外
- 攻方王手義務・受方応手による候補世界の絞り込み
- 覆面駒の移動、成、駒取り、持駒化、再打の個体追跡
- 残る全候補世界に対する証明済み詰み判定
- 指定手数の協力手順列挙
- JSON問題形式とCLI
- WebAssembly経由のWeb検討（JSON入力、候補世界・解答表示）
- Web盤面と両方の駒台での通常駒・覆面駒編集
- ▲・△を使った日本語の解答表示

Web版は [HiddenMate](https://springs022.github.io/hiddenmate/) で試せます。入力済みのサンプル問題は「覆面駒を検討」ボタンだけで実行できます。盤面と駒台をクリックして通常駒を移動でき、覆面駒は盤上または攻方・受方駒台に配置できます。候補駒種は選択せず、常に全14駒種です（駒台では合法な7駒種へ自動的に絞られます）。透明駒は今後実装します。

## CLI

Rust環境で次を実行します。

```console
cd rust
cargo run -p hiddenmate_cli -- ../examples/variable-rook-dragon.json
```

出力例：

```text
初形候補世界: 14
  V1: {Pawn, Lance, Knight, Silver, Gold, Bishop, Rook, King, ProPawn, ProLance, ProKnight, ProSilver, ProBishop, ProRook}
解数: 3
1: 82▲(64)
2: 82▲成(64)
3: 84▲(64)
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

HiddenMateは [fmrs](https://github.com/ogiekako/fmrs) を基にした派生ソフトウェアです。fmrsはKeigo Oka氏によりMIT Licenseで提供されています。本リポジトリではfmrsの著作権表示とMIT License本文を保持し、HiddenMateで追加した変更部分もMIT Licenseで提供します。

ライセンス本文は [LICENSE](LICENSE)、fmrsおよび利用している第三者ソフトウェアの帰属とライセンスは [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) を参照してください。Web版の配布物にも、これらの文書と主要なランタイム依存ライブラリのライセンス本文を同梱しています。
