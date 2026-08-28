import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, test, vi } from "vitest";
import { emptyBoard } from "../../model/board";
import Board from "./Board";

afterEach(() => {
  vi.useRealTimers();
});

function renderBoard() {
  const onClick = vi.fn();
  const onRightClick = vi.fn();
  render(
    <Board
      pieces={emptyBoard()}
      selected={undefined}
      onClick={onClick}
      onRightClick={onRightClick}
      squareLabel={([row, col]) => `${row}-${col}`}
    />,
  );
  return { onClick, onRightClick, square: screen.getByLabelText("0-0") };
}

describe("盤面のタッチ操作", () => {
  test("スクロールする距離を動いた場合は盤面クリックにしない", () => {
    vi.useFakeTimers();
    const { onClick, onRightClick, square } = renderBoard();

    fireEvent.touchStart(square, {
      touches: [{ clientX: 10, clientY: 10 }],
    });
    fireEvent.touchMove(square, {
      touches: [{ clientX: 10, clientY: 40 }],
    });
    fireEvent.touchEnd(square, { touches: [] });
    act(() => vi.advanceTimersByTime(500));

    expect(onClick).not.toHaveBeenCalled();
    expect(onRightClick).not.toHaveBeenCalled();
  });

  test("指を動かさないシングルタップは盤面クリックにする", () => {
    vi.useFakeTimers();
    const { onClick, onRightClick, square } = renderBoard();

    fireEvent.touchStart(square, {
      touches: [{ clientX: 10, clientY: 10 }],
    });
    fireEvent.touchEnd(square, { touches: [] });
    act(() => vi.advanceTimersByTime(350));

    expect(onClick).toHaveBeenCalledTimes(1);
    expect(onRightClick).not.toHaveBeenCalled();
  });

  test("ダブルタップは従来どおり右クリックにする", () => {
    vi.useFakeTimers();
    const { onClick, onRightClick, square } = renderBoard();

    for (let i = 0; i < 2; i += 1) {
      fireEvent.touchStart(square, {
        touches: [{ clientX: 10, clientY: 10 }],
      });
      fireEvent.touchEnd(square, { touches: [] });
    }
    act(() => vi.advanceTimersByTime(500));

    expect(onClick).not.toHaveBeenCalled();
    expect(onRightClick).toHaveBeenCalledTimes(1);
  });
});
