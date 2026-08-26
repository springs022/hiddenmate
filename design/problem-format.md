# 覆面駒問題JSON

```json
{
  "baseSfen": "9/9/kS7/N8/1L7/9/9/9/9 b - 1",
  "plies": 1,
  "handVariableMode": "distinguishable",
  "variables": [
    {
      "id": 1,
      "color": "black",
      "square": "64"
    }
  ]
}
```

## フィールド

- `baseSfen`: 覆面駒を除いた表示局面。受方持駒は書かない。
- `plies`: 指定手数。
- `handVariableMode`: 同じ駒台にある複数の覆面駒の識別方法。省略時は`distinguishable`。
  - `distinguishable`: V1、V2のように個体を指定して打つ。
  - `indistinguishable`: 個体を指定せずに打ち、どの個体だったかは後続の観測から推論する。
- `variables`: 初形の覆面駒。
- `id`: 手順中も変わらない覆面駒ID。
- `color`: `black`（攻方）または`white`（受方）。
- `square`: 盤上にある場合、将棋式の筋・段を2桁で表した配置マス。
- `inHand`: 駒台にある場合は`true`。`square`とは同時に指定しない。

候補は常に`P L N S G B R K +P +L +N +S +B +R`の全14駒種から始まる。駒台の覆面駒は、合法性により持駒になれる7駒種へ絞られる。旧形式との互換性のため`candidates`も読み込めるが、通常は指定しない。

駒台の例：

```json
{"id": 2, "color": "white", "inHand": true}
```

`baseSfen`の攻方持駒は明示情報として扱う。各候補世界の受方持駒は、標準駒数から盤上駒・攻方持駒・覆面駒を引いた残りとして自動的に補完する。
