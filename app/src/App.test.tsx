import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import App from "./App";

beforeEach(() => localStorage.clear());

test("renders HiddenMate title", () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: "HiddenMate" })).not.toBeNull();
  expect(
    screen.getByRole("heading", { name: "覆面駒（Variable）検討" }),
  ).not.toBeNull();
  expect(screen.getByRole("button", { name: "盤面・フォーム" })).not.toBeNull();
  expect(screen.getByLabelText("通常駒のbase SFEN")).not.toBeNull();
  expect(screen.getByText("受方駒台（自動補完）")).not.toBeNull();
  expect(screen.getByText("攻方駒台")).not.toBeNull();
  expect(screen.getByRole("button", { name: "単玉のみ" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "双玉のみ" })).not.toBeNull();
  expect(screen.queryByText("盤面をリセット")).toBeNull();
  expect(
    screen.getByRole("heading", { name: "覆面駒の新規追加" }),
  ).not.toBeNull();
  expect(screen.getByRole("heading", { name: "覆面駒一覧" })).not.toBeNull();
  expect(
    screen.getByRole("button", { name: "攻方持駒に追加（▲）" }),
  ).not.toBeNull();
  expect(
    screen.getByRole("button", { name: "受方持駒に追加（△）" }),
  ).not.toBeNull();
  expect(screen.queryByRole("heading", { name: /V1の設定/ })).toBeNull();
  expect(screen.queryByText("通常駒と覆面駒を初期化します。")).toBeNull();
  expect(
    screen.queryByText(/通常駒は盤面・駒台をクリックして移動できます/),
  ).toBeNull();
  expect(screen.getByRole("button", { name: "覆面駒を検討" })).not.toBeNull();
  expect(screen.getByRole("heading", { name: "Saved positions" })).not.toBeNull();
  expect(screen.queryByRole("heading", { name: "通常協力詰" })).toBeNull();
});

test("places variable solve controls below the variable control panel", () => {
  const { container } = render(<App />);
  const panel = container.querySelector(".variable-control-panel");
  const solveControls = container.querySelector(".variable-solve-controls");

  expect(panel).not.toBeNull();
  expect(solveControls).not.toBeNull();
  expect(panel!.contains(solveControls)).toBe(false);
  expect(panel!.nextElementSibling?.contains(solveControls)).toBe(true);
});

test("moves a selected standard piece by clicking the hand background", () => {
  const { container } = render(<App />);
  const silverSquare = screen.getByLabelText("83");
  const attackHand = screen.getByText("攻方駒台").closest(".variable-hand");

  expect(silverSquare.textContent).toContain("銀");
  expect(attackHand).not.toBeNull();
  fireEvent.click(silverSquare);
  fireEvent.click(attackHand!);

  expect(silverSquare.textContent).not.toContain("銀");
  expect(container.querySelector(".variable-hand-black")?.textContent).toContain(
    "銀1",
  );
});

test("saves the current variable position with a name", () => {
  render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "現在の局面を保存" }));
  fireEvent.change(screen.getByLabelText("保存名"), {
    target: { value: "テスト局面" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save" }));

  expect(screen.getByRole("button", { name: "テスト局面" })).not.toBeNull();
  expect(localStorage.getItem("hiddenmate_variable_saved_positions")).toContain(
    "テスト局面",
  );
});

test("loads the default single-king position from saved positions", () => {
  const { container } = render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "単玉のみ" }));

  expect(
    (screen.getByLabelText("通常駒のbase SFEN") as HTMLInputElement).value,
  ).toBe("4k4/9/9/9/9/9/9/9/9 b - 1");
  expect(container.querySelector(".variable-piece")).toBeNull();
});
