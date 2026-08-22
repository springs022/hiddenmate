#!/usr/bin/env bash
#
# smoke-status.sh — 単玉全駒煙(39枚)キャンペーンの軽量ステータス表示
#
# ローカルと GCP (fmrs-spot-N) で動いている `fmrs single-king-smoke` の
# ジョブを自動検出し、各ジョブの到達枚数・到達手数とメモリ状況を
# 1つの表にまとめて標準出力に出す。
#
# ジョブの探索は「実行中プロセスのコマンドラインから
# --seed-result-log <path>.jsonl を拾って <path>.log をログとみなす」方式。
# ジョブが $HOME 配下のどこにあっても(ジョブディレクトリが毎回変わっても)
# 手動でパスを書き換えずに追跡できる。
#
# 使い方:
#   ./scripts/smoke-status.sh                  # ローカル + GCP (fmrs-spot-3)
#   ./scripts/smoke-status.sh --local-only      # ローカルだけ
#   ./scripts/smoke-status.sh --gcp-only        # GCP だけ
#   ./scripts/smoke-status.sh --no-snapshot     # スナップショット保存をしない
#
# 環境変数:
#   GCP_INSTANCE    (default: fmrs-spot-3)
#   GCP_ZONE        (default: asia-northeast1-c)
#   SSH_TIMEOUT     (default: 200)   gcloud ssh のタイムアウト秒数
#   SNAPSHOT_FILE   (default: $HOME/scratch/fmrs/smoke_status_snapshots.log)
#
# 例:
#   GCP_INSTANCE=fmrs-spot-2 ./scripts/smoke-status.sh --gcp-only
#
# スナップショットについて:
#   各ジョブの最新 global_best_pieces 行を SFEN URL 込みで生の形のまま
#   $SNAPSHOT_FILE に追記する（表には出さない）。ジョブ側のログ/checkpoint が
#   /tmp 配下だと再起動で消える事故が過去に起きたため(2026-08-14,
#   fmrs-spot-3 の /tmp が systemd-tmpfiles の 'D' 指定で毎起動ごとに
#   クリアされる)、このスクリプトを定期的に流しておけば $HOME 配下に
#   最低限「そのジョブが到達していた具体的な局面」の記録が残る。

set -uo pipefail

GCP_INSTANCE="${GCP_INSTANCE:-fmrs-spot-3}"
GCP_ZONE="${GCP_ZONE:-asia-northeast1-c}"
SSH_TIMEOUT="${SSH_TIMEOUT:-200}"
SNAPSHOT_FILE="${SNAPSHOT_FILE:-$HOME/scratch/fmrs/smoke_status_snapshots.log}"

DO_LOCAL=1
DO_GCP=1
DO_SNAPSHOT=1
for a in "$@"; do
  case "$a" in
    --local-only) DO_GCP=0 ;;
    --gcp-only) DO_LOCAL=0 ;;
    --no-snapshot) DO_SNAPSHOT=0 ;;
    -h|--help) sed -n '2,32p' "$0"; exit 0 ;;
    *) echo "unknown option: $a (see --help)" >&2; exit 1 ;;
  esac
done

if [ "$DO_SNAPSHOT" = 1 ]; then
  mkdir -p "$(dirname "$SNAPSHOT_FILE")"
fi
snapshot() {
  # snapshot <where> <job> <full raw best line>
  [ "$DO_SNAPSHOT" = 1 ] || return 0
  [ -n "$3" ] || return 0
  printf '%s\t%s\t%s\t%s\n' "$(date -u +%FT%TZ)" "$1" "$2" "$3" >> "$SNAPSHOT_FILE"
}

echo "| 場所 | ジョブ | 到達 |"
echo "|---|---|---|"

if [ "$DO_LOCAL" = 1 ]; then
  n=$(ps aux | grep -c "[f]mrs single-king")
  logs=$(ps aux | grep "[f]mrs single-king" \
    | grep -oE -- '--seed-result-log [^ ]+\.jsonl' \
    | sed -E 's/--seed-result-log //; s/\.jsonl$/.log/' \
    | sort -u)
  if [ -z "$logs" ]; then
    printf '| ローカル | (プロセスなし) | - |\n'
  else
    while IFS= read -r log; do
      [ -f "$log" ] || continue
      base=$(basename "$log" .log)
      full=$(grep 'global_best_pieces' "$log" 2>/dev/null | tail -1)
      best=$(grep -oE 'global_best_pieces=[0-9]+ steps=[0-9]+' <<< "$full")
      printf '| ローカル | %s | %s |\n' "$base" "${best:--}"
      snapshot "local" "$base" "$full"
    done <<< "$logs"
  fi
  mem=$(free -g | awk '/^Mem:/ {print $3"GB/"$2"GB"}')
  printf '| ローカル | proc=%s mem=%s | |\n' "$n" "$mem"
fi

if [ "$DO_GCP" = 1 ]; then
  remote_out=$(timeout "$SSH_TIMEOUT" gcloud compute ssh "$GCP_INSTANCE" --zone "$GCP_ZONE" --command '
    n=$(ps aux | grep -c "[f]mrs single-king")
    logs=$(ps aux | grep "[f]mrs single-king" \
      | grep -oE -- "--seed-result-log [^ ]+\.jsonl" \
      | sed -E "s/--seed-result-log //; s/\.jsonl$/.log/" \
      | sort -u)
    for log in $logs; do
      [ -f "$log" ] || continue
      base=$(basename "$log" .log)
      full=$(grep "global_best_pieces" "$log" 2>/dev/null | tail -1)
      best=$(grep -oE "global_best_pieces=[0-9]+ steps=[0-9]+" <<< "$full")
      echo "LOG|$base|${best:--}|$full"
    done
    mem=$(free -g | awk "/^Mem:/ {print \$3\"GB/\"\$2\"GB\"}")
    echo "STAT|proc=$n mem=$mem|"
  ' 2>&1)

  if [ $? -ne 0 ] || ! grep -q '^\(LOG\|STAT\)|' <<< "$remote_out"; then
    printf '| GCP (%s) | ssh失敗/停止中 | - |\n' "$GCP_INSTANCE"
    echo "$remote_out" | tail -5 | sed 's/^/# /'
  else
    while IFS='|' read -r kind name best full; do
      case "$kind" in
        LOG)
          printf '| GCP (%s) | %s | %s |\n' "$GCP_INSTANCE" "$name" "$best"
          snapshot "gcp:$GCP_INSTANCE" "$name" "$full"
          ;;
        STAT) printf '| GCP (%s) | %s | |\n' "$GCP_INSTANCE" "$name" ;;
      esac
    done < <(grep '^\(LOG\|STAT\)|' <<< "$remote_out")
  fi
fi
