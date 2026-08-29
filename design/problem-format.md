# 覆面駒問題JSON

Web版の「JSON詳細編集」と`hiddenmate_cli`は同じ問題形式を使用する。現在の組み込みサンプルは次のとおり。

```json
{
  "baseSfen": "9/9/9/9/9/9/9/9/9 b 2r2b4g4s4n4l18p 1",
  "plies": 4,
  "rule": "helpSelfmate",
  "handVariableMode": "indistinguishable",
  "variables": [
    { "id": 1, "color": "black", "square": "14" },
    { "id": 2, "color": "black", "square": "26" },
    { "id": 3, "color": "white", "square": "33" },
    { "id": 4, "color": "white", "square": "56" }
  ]
}
```

同じ内容は[`examples/variable-help-selfmate.json`](../examples/variable-help-selfmate.json)に収録している。

## 問題フィールド

- `baseSfen`（必須）: 覆面駒を除いた通常駒の盤面・持駒を表すSFEN。攻方・受方の持駒も、記述した枚数をそのまま各候補世界で使用する。標準駒数のうちSFENに現れない駒は駒箱の在庫とみなし、覆面駒の正体を割り当てる際の候補にする。
- `plies`（必須）: 探索する最大手数。指定手数以下の解を、短い順に列挙する。
- `rule`（省略可）: 検討ルール。省略時は`helpmate`。
  - `helpmate`: 協力詰。攻方が受方玉を詰める。
  - `helpSelfmate`: 協力自玉詰。受方が攻方玉を詰める。
- `handVariableMode`（省略可）: 同じ駒台にある複数の覆面駒の識別方法。省略時は`indistinguishable`。
  - `distinguishable`: V1、V2のように個体を指定して打つ。
  - `indistinguishable`: 個体を指定せずに打ち、どの個体だったかは後続の観測から推論する。
- `variables`（必須）: 初形の覆面駒の配列。最大6枚。

`baseSfen`に書かれた手番は、ルールと`plies`から次のように決め直される。

| ルール | 奇数手 | 偶数手 |
| --- | --- | --- |
| `helpmate` | 攻方から | 受方から |
| `helpSelfmate` | 受方から | 攻方から |

探索終了時に詰み側の手番となる手数だけが対象になる。たとえば`helpSelfmate`の4手指定では、条件を満たせば2手解と4手解を列挙する。

## 覆面駒フィールド

- `id`（必須）: 問題内で一意な0以上65535以下の整数ID。通常は1から順に付け、確定前の手順ではV1、V2のように表示する。
- `color`（必須）: `black`（攻方）または`white`（受方）。
- `square`: 盤上にある場合、将棋式の筋・段を11から99までの2桁で指定する。
- `inHand`: 駒台にある場合は`true`を指定する。
- `candidates`（省略可）: 候補駒種を駒種コードの配列で限定する。省略時は全14駒種。

`square`と`inHand: true`はどちらか一方だけを指定する。IDと盤上の配置マスは重複できず、`square`には`baseSfen`の通常駒を置けない。駒台の所属は`color`と一致している必要がある。

候補駒種コードは次の14種類。

```text
P L N S G B R K +P +L +N +S +B +R
```

候補を省略しても、標準駒数の在庫と初形合法性によって不可能な候補世界は除外される。駒台の覆面駒は成駒・玉として打てないため、合法性により持駒になれる7駒種へ絞られる。

駒台の例：

```json
{ "id": 5, "color": "white", "inHand": true }
```

候補を飛・龍だけに限定する例：

```json
{
  "id": 1,
  "color": "black",
  "square": "64",
  "candidates": ["R", "+R"]
}
```

協力詰では受方玉がちょうど1枚必要で、攻方玉は0枚または1枚にできる。協力自玉詰では両方の玉がちょうど1枚必要となる。これらを含む初形合法性と矛盾しない割当が一つもなければエラーになる。
