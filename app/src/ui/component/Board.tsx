import { ReactNode, useEffect, useRef } from "react";
import { SELECTED_COLOR } from "../constants";
import * as model from "../../model";

const TOUCH_MOVE_THRESHOLD_PX = 10;

export default function Board(props: {
  pieces: model.Board;
  selected: [number, number] | undefined;
  onClick: (pos: [number, number]) => void;
  onRightClick: (pos: [number, number]) => void;
  overlay?: (pos: [number, number]) => ReactNode;
  squareLabel?: (pos: [number, number]) => string;
}) {
  const board = [];
  for (let row = 0; row < 9; row++) {
    const rowPieces = [];
    for (let col = 8; col >= 0; col--) {
      rowPieces.push(
        <Square
          key={col}
          onRightClick={() => props.onRightClick([row, col])}
          onClick={() => props.onClick([row, col])}
          piece={props.pieces[row][col]}
          overlay={props.overlay?.([row, col])}
          label={props.squareLabel?.([row, col])}
          selected={
            !!props.selected &&
            row === props.selected[0] &&
            col === props.selected[1]
          }
        />,
      );
    }
    board.push(<tr key={row}>{rowPieces}</tr>);
  }
  return (
    <table style={{ borderCollapse: "collapse" }}>
      <tbody>{board}</tbody>
    </table>
  );
}

function Square(props: {
  piece: model.Piece | undefined;
  overlay?: ReactNode;
  label?: string;
  selected: boolean;
  onClick: () => void;
  onRightClick: () => void;
}) {
  const lastTouchAt = useRef<number | undefined>(undefined);
  const singleTapTimer = useRef<number | undefined>(undefined);
  const suppressClickUntil = useRef(0);
  const touchStart = useRef<{ x: number; y: number } | undefined>(undefined);
  const touchMoved = useRef(false);

  useEffect(
    () => () => {
      if (singleTapTimer.current !== undefined) {
        window.clearTimeout(singleTapTimer.current);
      }
    },
    [],
  );

  const cancelPendingTap = () => {
    if (singleTapTimer.current !== undefined) {
      window.clearTimeout(singleTapTimer.current);
      singleTapTimer.current = undefined;
    }
    lastTouchAt.current = undefined;
  };

  const startTouch = (event: React.TouchEvent<HTMLTableCellElement>) => {
    const touch = event.touches[0];
    touchStart.current = touch
      ? { x: touch.clientX, y: touch.clientY }
      : undefined;
    touchMoved.current = event.touches.length !== 1;
  };

  const moveTouch = (event: React.TouchEvent<HTMLTableCellElement>) => {
    const start = touchStart.current;
    const touch = event.touches[0];
    if (!start || !touch) {
      touchMoved.current = true;
      return;
    }
    const dx = touch.clientX - start.x;
    const dy = touch.clientY - start.y;
    if (dx * dx + dy * dy > TOUCH_MOVE_THRESHOLD_PX ** 2) {
      touchMoved.current = true;
    }
  };

  const cancelTouch = () => {
    touchStart.current = undefined;
    touchMoved.current = false;
    suppressClickUntil.current = Date.now() + 500;
    cancelPendingTap();
  };

  const touchEnd = (event: React.TouchEvent<HTMLTableCellElement>) => {
    event.preventDefault();
    event.stopPropagation();
    const now = Date.now();
    suppressClickUntil.current = now + 500;
    touchStart.current = undefined;
    if (touchMoved.current) {
      touchMoved.current = false;
      cancelPendingTap();
      return;
    }
    if (lastTouchAt.current !== undefined && now - lastTouchAt.current <= 350) {
      cancelPendingTap();
      props.onRightClick();
      return;
    }

    lastTouchAt.current = now;
    singleTapTimer.current = window.setTimeout(() => {
      lastTouchAt.current = undefined;
      singleTapTimer.current = undefined;
      props.onClick();
    }, 350);
  };

  return (
    <td
      className="board-square"
      aria-label={props.label}
      onClick={() => {
        if (Date.now() >= suppressClickUntil.current) {
          props.onClick();
        }
      }}
      onTouchStart={startTouch}
      onTouchMove={moveTouch}
      onTouchEnd={touchEnd}
      onTouchCancel={cancelTouch}
      onContextMenu={(e) => {
        e.preventDefault();
        e.stopPropagation();
        props.onRightClick();
      }}
      style={{
        padding: 0,
        border: "1px solid black",
        backgroundColor: props.selected ? SELECTED_COLOR : "white",
        fontSize: "1.5em",
        verticalAlign: "middle",
        touchAction: "manipulation",
        userSelect: "none",
      }}
    >
      {props.overlay ?? (props.piece ? pieceString(props.piece) : "")}
    </td>
  );
}

function pieceString(p: model.Piece) {
  const color = p === "O" ? "black" : p.color;
  const letter = p === "O" ? "⬤" : MAPPING[p.kind][p.promoted ? 1 : 0];
  return (
    <div
      style={{
        transform: color === "black" ? "rotate(0)" : "rotate(180deg)",
        textAlign: "center",
      }}
    >
      {letter}
    </div>
  );
}

const MAPPING: { [K in model.Kind]: [string, string] } = {
  P: ["歩", "と"],
  L: ["香", "杏"],
  N: ["桂", "圭"],
  S: ["銀", "全"],
  G: ["金", ""],
  B: ["角", "馬"],
  R: ["飛", "龍"],
  K: ["玉", ""],
};
