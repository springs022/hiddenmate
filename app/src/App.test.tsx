import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import App from "./App";

beforeEach(() => localStorage.clear());

test("renders HiddenMate title", () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: "HiddenMate" })).not.toBeNull();
  expect(
    screen.getByRole("heading", { name: "覆面駒入りの協力詰／協力自玉詰" }),
  ).not.toBeNull();
  expect(screen.getByRole("button", { name: "盤面・フォーム" })).not.toBeNull();
  expect(screen.getByLabelText("通常駒のbase SFEN")).not.toBeNull();
  expect(screen.queryByText("受方駒台")).toBeNull();
  expect(screen.queryByText("攻方駒台")).toBeNull();
  expect(screen.getByRole("button", { name: "単玉のみ" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "双玉のみ" })).not.toBeNull();
  expect(screen.queryByText("盤面をリセット")).toBeNull();
  expect(
    screen.getByRole("heading", { name: "覆面駒の新規追加" }),
  ).not.toBeNull();
  expect(screen.getByRole("heading", { name: "覆面駒一覧" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "攻方持駒に追加" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "受方持駒に追加" })).not.toBeNull();
  expect(screen.queryByRole("heading", { name: /V1の設定/ })).toBeNull();
  expect(screen.queryByText("通常駒と覆面駒を初期化します。")).toBeNull();
  expect(
    screen.queryByText(/通常駒は盤面・駒台をクリックして移動できます/),
  ).toBeNull();
  expect(screen.getByRole("button", { name: "覆面駒を検討" })).not.toBeNull();
  expect(
    screen.getByRole("heading", { name: "Saved positions" }),
  ).not.toBeNull();
  expect(screen.queryByRole("heading", { name: "通常協力詰" })).toBeNull();
});

test("places the editor, variable controls, and solve results in three columns", () => {
  const { container } = render(<App />);
  const panel = container.querySelector(".variable-control-panel");
  const solveControls = container.querySelector(".variable-solve-controls");
  const columns = container.querySelectorAll(
    ".variable-three-column-layout > .col-xl-4",
  );

  expect(panel).not.toBeNull();
  expect(solveControls).not.toBeNull();
  expect(panel!.contains(solveControls)).toBe(false);
  expect(columns).toHaveLength(3);
  expect(columns[1].contains(panel)).toBe(true);
  expect(columns[2].contains(solveControls)).toBe(true);
});

test("moves a selected standard piece by clicking the hand background", () => {
  const { container } = render(<App />);
  const silverSquare = screen.getByLabelText("83");
  const attackHand = container.querySelector(".variable-hand-black");

  expect(silverSquare.textContent).toContain("銀");
  expect(attackHand).not.toBeNull();
  fireEvent.click(silverSquare);
  fireEvent.click(attackHand!);

  expect(silverSquare.textContent).not.toContain("銀");
  expect(
    container.querySelector(".variable-hand-black")?.textContent,
  ).toContain("銀1");
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

test("keeps a newly added variable unselected", () => {
  const { container } = render(<App />);

  fireEvent.click(screen.getByRole("button", { name: "攻方持駒に追加" }));

  expect(container.querySelectorAll(".variable-piece-selected")).toHaveLength(
    0,
  );
  expect(screen.getByRole("button", { name: /▲V2/ }).className).toContain(
    "btn-outline-primary",
  );

  fireEvent.click(screen.getByText("▲V2"));
  expect(
    container.querySelectorAll(".variable-hand-piece-selected"),
  ).toHaveLength(1);
});

test("allows at most six variables and does not highlight a hand", () => {
  const { container } = render(<App />);
  const addBlack = screen.getByRole("button", {
    name: "攻方持駒に追加",
  });
  const addWhite = screen.getByRole("button", {
    name: "受方持駒に追加",
  });

  for (let i = 0; i < 5; i += 1) {
    fireEvent.click(addBlack);
  }

  expect((addBlack as HTMLButtonElement).disabled).toBe(true);
  expect((addWhite as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText("▲V6")).not.toBeNull();
  expect(screen.queryByText("▲V7")).toBeNull();

  fireEvent.click(screen.getByLabelText("64 V1"));
  expect(container.querySelector(".variable-hand-drop-target")).toBeNull();
});

test("toggles a board variable and clears selection after moving it", () => {
  const { container } = render(<App />);
  const variableSquare = screen.getByLabelText("64 V1");

  fireEvent.click(variableSquare);
  expect(container.querySelectorAll(".variable-piece-selected")).toHaveLength(
    1,
  );
  fireEvent.click(variableSquare);
  expect(container.querySelectorAll(".variable-piece-selected")).toHaveLength(
    0,
  );

  fireEvent.click(variableSquare);
  fireEvent.click(screen.getByLabelText("55"));
  expect(screen.getByLabelText("55 V1")).not.toBeNull();
  expect(container.querySelectorAll(".variable-piece-selected")).toHaveLength(
    0,
  );
});

test("rotates a white board variable like a standard white piece", () => {
  render(<App />);

  fireEvent.click(screen.getByRole("button", { name: "受方持駒に追加" }));
  fireEvent.click(screen.getByText("△V2"));
  fireEvent.click(screen.getByLabelText("55"));

  expect(
    (screen.getByLabelText("55 V2").firstElementChild as HTMLElement).style
      .transform,
  ).toBe("rotate(180deg)");
});

test("reverses a board variable owner by right click or double tap", () => {
  render(<App />);
  const variableSquare = screen.getByLabelText("64 V1");

  fireEvent.contextMenu(variableSquare);
  expect(variableSquare.textContent).toContain("△V1");
  expect(
    (variableSquare.firstElementChild as HTMLElement).style.transform,
  ).toBe("rotate(180deg)");

  fireEvent.touchEnd(variableSquare);
  fireEvent.touchEnd(variableSquare);
  expect(variableSquare.textContent).toContain("▲V1");
  expect(
    (variableSquare.firstElementChild as HTMLElement).style.transform,
  ).toBe("rotate(0deg)");
});

test("does not show nothing when an otherwise empty hand has a variable", () => {
  const { container } = render(<App />);
  const attackHand = container.querySelector(".variable-hand-black");

  expect(attackHand?.textContent).toContain("なし");
  fireEvent.click(screen.getByRole("button", { name: "攻方持駒に追加" }));

  expect(attackHand?.textContent).toContain("▲V2");
  expect(attackHand?.textContent).not.toContain("なし");
});

test("shows a spinner before starting a variable solve", async () => {
  jest.useFakeTimers();
  render(<App />);

  fireEvent.click(screen.getByRole("button", { name: "覆面駒を検討" }));

  expect(screen.getByRole("status")).not.toBeNull();
  expect(screen.getByRole("button", { name: "検討中…" })).not.toBeNull();

  await act(async () => {
    jest.runOnlyPendingTimers();
  });
  jest.useRealTimers();
});

test("allows clearing the plies field before entering a new value", () => {
  render(<App />);
  const plies = screen.getByLabelText("手数") as HTMLInputElement;

  fireEvent.change(plies, { target: { value: "" } });
  expect(plies.value).toBe("");

  fireEvent.change(plies, { target: { value: "3" } });
  expect(plies.value).toBe("3");
});

test("selects help-selfmate and includes the rule in problem JSON", () => {
  render(<App />);

  const helpmate = screen.getByLabelText("協力詰") as HTMLInputElement;
  const helpSelfmate = screen.getByLabelText("協力自玉詰") as HTMLInputElement;
  expect(helpmate.checked).toBe(true);

  fireEvent.click(helpSelfmate);
  expect(helpSelfmate.checked).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "JSON詳細編集" }));

  expect(
    (screen.getByLabelText("問題JSON") as HTMLTextAreaElement).value,
  ).toContain('"rule": "helpSelfmate"');
});

test("treats a board double tap like a right click", () => {
  render(<App />);
  const silverSquare = screen.getByLabelText("83");

  fireEvent.touchEnd(silverSquare);
  fireEvent.touchEnd(silverSquare);

  expect(silverSquare.textContent).toContain("全");
});
