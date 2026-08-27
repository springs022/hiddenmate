import React, { useState } from "react";
import * as types from "../types";

export type ShiftDirection = "up" | "down" | "left" | "right";

type ShifterProps = {
  children: React.ReactNode;
} & (
  | { dispatch: types.Dispatcher; onShift?: never }
  | {
      dispatch?: never;
      onShift: (direction: ShiftDirection) => void;
    }
);

export function Shifter(props: ShifterProps) {
  const cursors = new Map<ShiftDirection, React.ReactNode>();
  for (const dir of ["up", "down", "left", "right"] as const) {
    cursors.set(
      dir,
      <Cursor
        direction={dir}
        onClick={() => {
          if (props.onShift) {
            props.onShift(dir);
          } else {
            props.dispatch?.({
              ty: "shift",
              dir,
            });
          }
        }}
      />
    );
  }

  return (
    <div className="fit-content shifter">
      {cursors.get("up")}
      {cursors.get("left")}
      <div className="shifter-content">{props.children}</div>
      {cursors.get("right")}
      {cursors.get("down")}
    </div>
  );
}

function Cursor(props: { direction: ShiftDirection; onClick: () => void }) {
  const [focused, setFocused] = useState(false);

  const letter = {
    up: "△",
    down: "▽",
    left: "◁",
    right: "▷",
  }[props.direction];
  return (
    <div
      className={`shifter-cursor shifter-cursor-${props.direction} text-secondary d-flex align-items-center justify-content-center user-select-none`}
    >
      <span
        title={`${props.direction} shift`}
        onClick={props.onClick}
        onMouseEnter={() => setFocused(true)}
        onMouseLeave={() => setFocused(false)}
        style={{
          opacity: focused ? 1 : 0.65,
          fontSize: focused ? "1em" : "0.5em",
        }}
      >
        {letter}
      </span>
    </div>
  );
}
