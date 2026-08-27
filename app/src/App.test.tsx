import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";

vi.mock("./wasm_api", () => ({
  Algorithm: {},
  BackwardSearch: class {},
  Solver: class {},
  check_one_way_mate: vi.fn(),
  is_white_in_check: vi.fn(() => false),
  solve_variable_problem: vi.fn(),
}));

import App from "./App";

const workerInstances: MockWorker[] = [];

class MockWorker {
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  onmessageerror: ((event: MessageEvent) => void) | null = null;
  postMessage = vi.fn();
  terminate = vi.fn();

  constructor() {
    workerInstances.push(this);
  }
}

beforeAll(() => {
  Object.defineProperty(globalThis, "Worker", {
    configurable: true,
    writable: true,
    value: MockWorker,
  });
});

beforeEach(() => {
  localStorage.clear();
  workerInstances.length = 0;
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: vi.fn().mockResolvedValue(undefined) },
  });
});

test("renders HiddenMate title", () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: "HiddenMate" })).not.toBeNull();
  expect(screen.getByRole("heading", { name: "覆面駒" })).not.toBeNull();
  expect(screen.getByText("覆面駒版 β")).not.toBeNull();
  expect(screen.getByRole("heading", { name: "透明駒" })).not.toBeNull();
  expect(screen.getByText(/透明駒の検討機能は開発中です/)).not.toBeNull();
  expect(
    screen.queryByText(/現在は覆面駒の検討機能をご利用ください/),
  ).toBeNull();
  expect(screen.queryByRole("heading", { name: "はじめに" })).toBeNull();
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
  expect(screen.getByRole("button", { name: "検討" })).not.toBeNull();
  expect(screen.getByRole("heading", { name: "保存局面" })).not.toBeNull();
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

test("clears piece selections when clicking outside the board and hands", () => {
  const { container } = render(<App />);
  const silverSquare = screen.getByLabelText("83");
  const variableButton = screen.getByRole("button", { name: /▲V1 64/ });
  const outside = screen.getByRole("heading", { name: "覆面駒" });

  fireEvent.click(silverSquare);
  expect(silverSquare.style.backgroundColor).not.toBe("white");
  fireEvent.click(outside);
  expect(silverSquare.style.backgroundColor).toBe("white");

  fireEvent.click(variableButton);
  expect(container.querySelectorAll(".variable-piece-selected")).toHaveLength(1);
  fireEvent.click(outside);
  expect(container.querySelectorAll(".variable-piece-selected")).toHaveLength(0);
});

test("saves the current variable position with a name", () => {
  render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "現在の局面を保存" }));
  fireEvent.change(screen.getByLabelText("保存名"), {
    target: { value: "テスト局面" },
  });
  fireEvent.click(screen.getByRole("button", { name: "保存" }));

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

test("shows progress and can cancel and restart a variable solve", () => {
  render(<App />);

  fireEvent.click(screen.getByRole("button", { name: "検討" }));

  expect(screen.getByRole("status")).not.toBeNull();
  expect(screen.getByRole("button", { name: "中断" })).not.toBeNull();
  expect(workerInstances).toHaveLength(1);
  expect(workerInstances[0].postMessage).not.toHaveBeenCalled();

  act(() => {
    workerInstances[0].onmessage?.({
      data: { type: "ready" },
    } as MessageEvent);
  });
  expect(workerInstances[0].postMessage).toHaveBeenCalledTimes(1);

  fireEvent.click(screen.getByRole("button", { name: "中断" }));

  expect(workerInstances[0].terminate).toHaveBeenCalledTimes(1);
  expect(screen.queryByRole("status")).toBeNull();
  expect(screen.getByRole("button", { name: "検討" })).not.toBeNull();

  fireEvent.click(screen.getByRole("button", { name: "検討" }));

  expect(workerInstances).toHaveLength(2);
  expect(screen.getByRole("button", { name: "中断" })).not.toBeNull();

  fireEvent.click(screen.getByRole("button", { name: "JSON詳細編集" }));

  expect(workerInstances[1].terminate).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("button", { name: "検討" })).not.toBeNull();
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

test("defaults to indistinguishable hand variables and can select distinguishable", () => {
  render(<App />);

  const distinguishable = screen.getByLabelText("区別する") as HTMLInputElement;
  const indistinguishable = screen.getByLabelText(
    "区別しない",
  ) as HTMLInputElement;
  expect(indistinguishable.checked).toBe(true);

  fireEvent.click(distinguishable);
  expect(distinguishable.checked).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "JSON詳細編集" }));

  expect(
    (screen.getByLabelText("問題JSON") as HTMLTextAreaElement).value,
  ).toContain('"handVariableMode": "distinguishable"');
});

test("treats a board double tap like a right click", () => {
  render(<App />);
  const silverSquare = screen.getByLabelText("83");

  fireEvent.touchEnd(silverSquare);
  fireEvent.touchEnd(silverSquare);

  expect(silverSquare.textContent).toContain("全");
});

test("copies the generated problem JSON", async () => {
  render(<App />);

  fireEvent.click(screen.getByRole("button", { name: "JSON詳細編集" }));
  fireEvent.click(screen.getByRole("button", { name: "問題JSONをコピー" }));

  expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
    expect.stringContaining('"baseSfen"'),
  );
  expect(await screen.findByText("コピーしました")).not.toBeNull();
});

test("copies a formatted solve result", async () => {
  render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "検討" }));
  act(() => {
    workerInstances[0].onmessage?.({ data: { type: "ready" } } as MessageEvent);
    workerInstances[0].onmessage?.({
      data: {
        type: "solved",
        requestId: 1,
        responseJson: JSON.stringify({
          worldCount: 1,
          candidates: [{ id: 1, kinds: ["R"] }],
          solutions: [["82▲(64)"]],
        }),
      },
    } as MessageEvent);
  });

  fireEvent.click(await screen.findByRole("button", { name: "解答をコピー" }));

  expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
    expect.stringContaining("1. 82▲(64) まで 1手"),
  );
});
