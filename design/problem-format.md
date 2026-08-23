# 覆面駒問題JSON

```json
{
  "baseSfen": "9/9/kS7/N8/1L7/9/9/9/9 b - 1",
  "plies": 1,
  "variables": [
    {
      "id": 1,
      "color": "black",
      "square": "64",
      "candidates": ["R", "+R"]
    }
  ]
}
```

## フィールド

- `baseSfen`: 覆面駒を除いた表示局面。受方持駒は書かない。
- `plies`: 指定手数。
- `variables`: 初形の覆面駒。
- `id`: 手順中も変わらない覆面駒ID。
- `color`: `black`（攻方）または`white`（受方）。
- `square`: 将棋式の筋・段を2桁で表した配置マス。
- `candidates`: 候補駒種。

駒種には`P L N S G B R K +P +L +N +S +B +R`を使う。

`baseSfen`の攻方持駒は明示情報として扱う。各候補世界の受方持駒は、標準駒数から盤上駒・攻方持駒・覆面駒を引いた残りとして自動的に補完する。
