import React from "react";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
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

function placeSilverAt83() {
  fireEvent.click(screen.getByText("銀"));
  fireEvent.click(screen.getByLabelText("83"));
  return screen.getByLabelText("83");
}

test("renders HiddenMate title", () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: "HiddenMate" })).not.toBeNull();
  const homepageLink = screen.getByRole("link", { name: "フェアリーの風音" });
  expect(homepageLink.getAttribute("href")).toBe("https://tsume-springs.com/");
  expect(homepageLink.getAttribute("target")).toBe("_blank");
  expect(homepageLink.getAttribute("rel")).toBe("noopener noreferrer");
  expect(screen.getByText("覆面駒・透明駒の検討")).not.toBeNull();
  expect(screen.getByRole("heading", { name: "覆面駒" })).not.toBeNull();
  expect(
    screen.getByRole("heading", { name: "透明駒（駒種指定）" }),
  ).not.toBeNull();
  expect(screen.getByRole("button", { name: "入力を開く" })).not.toBeNull();
  expect(screen.queryByText("覆面駒版 β")).toBeNull();
  expect(screen.getByRole("heading", { name: "透明駒" })).not.toBeNull();
  expect(screen.getByText(/透明駒の検討機能は開発中です/)).not.toBeNull();
  expect(
    screen.queryByText(/現在は覆面駒の検討機能をご利用ください/),
  ).toBeNull();
  expect(screen.queryByRole("heading", { name: "はじめに" })).toBeNull();
  expect(screen.getByRole("button", { name: "盤面・フォーム" })).not.toBeNull();
  expect(screen.getByLabelText("通常駒のbase SFEN")).not.toBeNull();
  expect(screen.getByText("駒箱")).not.toBeNull();
  expect(screen.queryByText("受方駒台")).toBeNull();
  expect(screen.queryByText("攻方駒台")).toBeNull();
  expect(screen.getByRole("button", { name: "単玉のみ" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "双玉のみ" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "すべて駒箱" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "サンプル" })).not.toBeNull();
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

test("configures at most two known-kind invisible pieces", () => {
  const { container } = render(<App />);
  expect((screen.getByLabelText("最大解数") as HTMLInputElement).value).toBe("20");
  const openButton = screen.getByRole("button", { name: "入力を開く" });
  expect(openButton.closest(".card-header")).not.toBeNull();
  expect(openButton.textContent).toBe("");
  fireEvent.click(openButton);
  const solver = container.querySelector(".known-invisible-solver")!;
  const panel = within(solver as HTMLElement);
  const description = panel.getByText(
    "駒種と所属が分かっていて、位置だけが不明な透明駒です。合計2枚まで指定できます。",
  );
  const modeGroup = panel.getByRole("group", {
    name: "透明駒（駒種指定）の問題入力方法",
  });
  expect(
    description.compareDocumentPosition(modeGroup) &
      Node.DOCUMENT_POSITION_FOLLOWING,
  ).not.toBe(0);
  expect(panel.getByRole("button", { name: "盤面・フォーム" })).not.toBeNull();
  expect(panel.getByLabelText("手数")).not.toBeNull();
  expect((panel.getByLabelText("最大解数") as HTMLInputElement).value).toBe("20");
  expect(panel.queryByText("最大手数")).toBeNull();
  expect(panel.getByRole("button", { name: "双玉のみ" })).not.toBeNull();
  expect(
    panel.queryByRole("button", { name: "双玉・残り全部受方持駒" }),
  ).toBeNull();
  expect(solver.querySelector(".variable-solve-controls")).not.toBeNull();
  expect(solver.querySelector(".variable-solve-actions")).not.toBeNull();
  expect(
    (panel.getByLabelText("通常駒のbase SFEN") as HTMLInputElement).value,
  ).toBe("4k4/9/9/9/9/9/9/9/4K4 b 2r2b4g4s4n4l18p 1");
  expect(solver.querySelector(".variable-piece-box")?.textContent).toContain("なし");
  const owner = panel.getByLabelText("所属");
  const kind = panel.getByLabelText("駒種");
  const add = panel.getByRole("button", { name: "透明駒を追加" });
  fireEvent.change(owner, { target: { value: "white" } });
  fireEvent.change(kind, { target: { value: "K" } });
  fireEvent.click(add);
  fireEvent.change(owner, { target: { value: "black" } });
  fireEvent.change(kind, { target: { value: "R" } });
  fireEvent.click(add);

  expect((add as HTMLButtonElement).disabled).toBe(true);
  expect(panel.getByText("受方 玉 ×1")).not.toBeNull();
  expect(panel.getByText("攻方 飛 ×1")).not.toBeNull();
  expect(panel.getByRole("button", { name: "受方玉を1枚削除" })).not.toBeNull();
  fireEvent.click(panel.getByRole("button", { name: "JSON詳細編集" }));
  const problemJson = panel.getByLabelText("問題JSON") as HTMLTextAreaElement;
  expect(problemJson.value).toContain('"kind": "K"');
  expect(problemJson.value).toContain('"kind": "R"');
});

test("shows known invisible rule, plies, and piece summary above counts", async () => {
  const { container } = render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "入力を開く" }));
  const solver = container.querySelector(".known-invisible-solver")!;
  const panel = within(solver as HTMLElement);
  fireEvent.click(panel.getByRole("button", { name: "JSON詳細編集" }));
  fireEvent.change(panel.getByLabelText("問題JSON"), {
    target: {
      value: JSON.stringify({
        baseSfen: "7k1/9/7K1/9/9/9/9/9/9 b - 1",
        plies: 5,
        rule: "helpmate",
        invisibles: [
          { color: "white", kind: "+R", count: 1 },
          { color: "black", kind: "B", count: 1 },
        ],
      }),
    },
  });
  fireEvent.click(panel.getByRole("button", { name: "検討" }));
  act(() => {
    workerInstances[0].onmessage?.({ data: { type: "ready" } } as MessageEvent);
    workerInstances[0].onmessage?.({
      data: {
        type: "solved",
        requestId: 1,
        responseJson: JSON.stringify({
          worldCount: 71,
          solutions: [
            ["13玉", "23歩", "同X", "11玉", "22香成"],
            ...Array.from({ length: 5 }, () => ["X"]),
          ],
        }),
      },
    } as MessageEvent);
  });

  const ruleSummary = await panel.findByText("協力詰 5手");
  const pieceSummary = panel.getByText("攻方透明角1枚、受方透明龍1枚");
  const countsSummary = panel.getByText(/初形候補世界:/);
  expect(
    ruleSummary.compareDocumentPosition(pieceSummary) &
      Node.DOCUMENT_POSITION_FOLLOWING,
  ).not.toBe(0);
  expect(
    pieceSummary.compareDocumentPosition(countsSummary) &
      Node.DOCUMENT_POSITION_FOLLOWING,
  ).not.toBe(0);
  expect(countsSummary.textContent).toContain("71");
  expect(countsSummary.textContent).toContain("6");
  expect(
    panel.getByText("13玉 23歩 同X 11玉 22香成 まで 5手"),
  ).not.toBeNull();
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
  const pieceBox = container.querySelector(".variable-piece-box");
  const whiteHand = container.querySelector(".variable-hand-white");
  expect(columns[0].contains(pieceBox)).toBe(true);
  expect(pieceBox?.classList.contains("variable-hand")).toBe(true);
  expect(pieceBox?.parentElement).toBe(whiteHand?.parentElement);
  expect(columns[1].contains(panel)).toBe(true);
  expect(columns[2].contains(solveControls)).toBe(true);
  const baseSfen = screen.getByLabelText("通常駒のbase SFEN");
  const savedPositions = container.querySelector(".variable-saved-positions");
  expect(columns[1].contains(baseSfen)).toBe(true);
  expect(
    Boolean(
      baseSfen.compareDocumentPosition(savedPositions!) &
      Node.DOCUMENT_POSITION_FOLLOWING,
    ),
  ).toBe(true);
});

test("starts with the help-selfmate sample", () => {
  const { container } = render(<App />);
  const pieceBox = container.querySelector(".variable-piece-box");
  const whiteHand = container.querySelector(".variable-hand-white");

  expect(pieceBox?.textContent).toContain("玉2");
  expect(whiteHand?.textContent).toContain("飛2");
  expect(whiteHand?.textContent).toContain("歩18");
  expect(
    (screen.getByLabelText("通常駒のbase SFEN") as HTMLInputElement).value,
  ).toBe("9/9/9/9/9/9/9/9/9 b 2r2b4g4s4n4l18p 1");
  expect((screen.getByLabelText("手数") as HTMLInputElement).value).toBe("4");
  expect(
    (screen.getByLabelText("協力自玉詰") as HTMLInputElement).checked,
  ).toBe(true);
  expect(screen.getByLabelText("14 V1")).not.toBeNull();
  expect(screen.getByLabelText("26 V2")).not.toBeNull();
  expect(screen.getByLabelText("33 V3")).not.toBeNull();
  expect(screen.getByLabelText("56 V4")).not.toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "JSON詳細編集" }));
  expect(
    JSON.parse((screen.getByLabelText("問題JSON") as HTMLTextAreaElement).value),
  ).toEqual({
    baseSfen: "9/9/9/9/9/9/9/9/9 b 2r2b4g4s4n4l18p 1",
    plies: 4,
    rule: "helpSelfmate",
    handVariableMode: "indistinguishable",
    variables: [
      { id: 1, color: "black", square: "14" },
      { id: 2, color: "black", square: "26" },
      { id: 3, color: "white", square: "33" },
      { id: 4, color: "white", square: "56" },
    ],
  });
});

test("moves a selected standard piece by clicking the hand background", () => {
  const { container } = render(<App />);
  const silverSquare = placeSilverAt83();
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

test("moves the defending king directly to the piece box", () => {
  const { container } = render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "双玉のみ" }));
  const kingSquare = screen.getByLabelText("51");
  const pieceBox = container.querySelector(".variable-piece-box");

  expect(kingSquare.textContent).toContain("玉");
  expect(pieceBox).not.toBeNull();
  fireEvent.click(kingSquare);
  fireEvent.click(pieceBox!);

  expect(kingSquare.textContent).not.toContain("玉");
  expect(pieceBox?.textContent).toContain("玉1");
});

test("moves the defending king to the piece box via the defending hand", () => {
  const { container } = render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "双玉のみ" }));
  const kingSquare = screen.getByLabelText("51");
  const defendingHand = container.querySelector(".variable-hand-white");
  const pieceBox = container.querySelector(".variable-piece-box");

  expect(kingSquare.textContent).toContain("玉");
  expect(defendingHand).not.toBeNull();
  fireEvent.click(kingSquare);
  fireEvent.click(defendingHand!);

  expect(kingSquare.textContent).not.toContain("玉");
  expect(defendingHand?.textContent).not.toContain("玉");
  expect(pieceBox?.textContent).toContain("玉1");
});

test("clears piece selections when clicking outside the board and hands", () => {
  const { container } = render(<App />);
  const silverSquare = placeSilverAt83();
  const variableButton = screen.getByRole("button", { name: /▲V1 14/ });
  const outside = screen.getByRole("heading", { name: "覆面駒" });

  fireEvent.click(silverSquare);
  expect(silverSquare.style.backgroundColor).not.toBe("white");
  fireEvent.click(outside);
  expect(silverSquare.style.backgroundColor).toBe("white");

  fireEvent.click(variableButton);
  expect(container.querySelectorAll(".variable-piece-selected")).toHaveLength(
    1,
  );
  fireEvent.click(outside);
  expect(container.querySelectorAll(".variable-piece-selected")).toHaveLength(
    0,
  );
});

test("shifts standard and variable pieces together", () => {
  render(<App />);
  placeSilverAt83();

  expect(screen.getByLabelText("83").textContent).toContain("銀");
  expect(screen.getByLabelText("14 V1").textContent).toContain("V1");

  fireEvent.click(screen.getByTitle("right shift"));

  expect(screen.getByLabelText("73").textContent).toContain("銀");
  expect(screen.getByLabelText("94 V1").textContent).toContain("V1");
  expect(screen.getByLabelText("83").textContent).not.toContain("銀");
  expect(screen.getByLabelText("14").textContent).not.toContain("V1");
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
  ).toBe("4k4/9/9/9/9/9/9/9/9 b 2r2b4g4s4n4l18p 1");
  expect(container.querySelector(".variable-piece")).toBeNull();
});

test("loads the all-pieces-in-box position from saved positions", () => {
  const { container } = render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "すべて駒箱" }));

  expect(
    (screen.getByLabelText("通常駒のbase SFEN") as HTMLInputElement).value,
  ).toBe("9/9/9/9/9/9/9/9/9 b - 1");
  expect(container.querySelector(".variable-piece")).toBeNull();
  expect(container.querySelector(".variable-piece-box")?.textContent).toContain(
    "玉2",
  );
  expect(container.querySelector(".variable-piece-box")?.textContent).toContain(
    "飛2",
  );
  expect(container.querySelector(".variable-piece-box")?.textContent).toContain(
    "歩18",
  );
});

test("loads the sample below the all-pieces-in-box position", () => {
  render(<App />);
  const allPiecesInBox = screen.getByRole("button", { name: "すべて駒箱" });
  const sample = screen.getByRole("button", { name: "サンプル" });

  expect(
    Boolean(
      allPiecesInBox.compareDocumentPosition(sample) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ),
  ).toBe(true);
  fireEvent.click(allPiecesInBox);
  fireEvent.click(sample);

  expect(
    (screen.getByLabelText("通常駒のbase SFEN") as HTMLInputElement).value,
  ).toBe("9/9/9/9/9/9/9/9/9 b 2r2b4g4s4n4l18p 1");
  expect((screen.getByLabelText("手数") as HTMLInputElement).value).toBe("4");
  expect(
    (screen.getByLabelText("協力自玉詰") as HTMLInputElement).checked,
  ).toBe(true);
  expect(screen.getByLabelText("14 V1")).not.toBeNull();
  expect(screen.getByLabelText("26 V2")).not.toBeNull();
  expect(screen.getByLabelText("33 V3")).not.toBeNull();
  expect(screen.getByLabelText("56 V4")).not.toBeNull();
});

test("migrates saved positions while refreshing built-in defaults", () => {
  localStorage.setItem(
    "hiddenmate_variable_saved_positions",
    JSON.stringify({
      version: 2,
      positions: [
        {
          name: "単玉のみ",
          problem: {
            baseSfen: "9/9/9/9/9/9/9/9/9 b - 1",
            plies: 1,
            variables: [],
          },
        },
        {
          name: "ユーザー局面",
          problem: {
            baseSfen: "4k4/9/9/9/9/9/9/9/9 b - 1",
            plies: 1,
            variables: [],
          },
        },
      ],
    }),
  );

  render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "単玉のみ" }));

  expect(
    (screen.getByLabelText("通常駒のbase SFEN") as HTMLInputElement).value,
  ).toBe("4k4/9/9/9/9/9/9/9/9 b 2r2b4g4s4n4l18p 1");
  expect(screen.getByRole("button", { name: "ユーザー局面" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "すべて駒箱" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "サンプル" })).not.toBeNull();
  expect(localStorage.getItem("hiddenmate_variable_saved_positions")).toContain(
    '"version":3',
  );
});

test("keeps a newly added variable unselected", () => {
  const { container } = render(<App />);

  fireEvent.click(screen.getByRole("button", { name: "攻方持駒に追加" }));

  expect(container.querySelectorAll(".variable-piece-selected")).toHaveLength(
    0,
  );
  expect(screen.getByRole("button", { name: /▲V5/ }).className).toContain(
    "btn-outline-primary",
  );

  fireEvent.click(screen.getByText("▲V5"));
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

  for (let i = 0; i < 2; i += 1) {
    fireEvent.click(addBlack);
  }

  expect((addBlack as HTMLButtonElement).disabled).toBe(true);
  expect((addWhite as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText("▲V6")).not.toBeNull();
  expect(screen.queryByText("▲V7")).toBeNull();

  fireEvent.click(screen.getByLabelText("14 V1"));
  expect(container.querySelector(".variable-hand-drop-target")).toBeNull();
});

test("toggles a board variable and clears selection after moving it", () => {
  const { container } = render(<App />);
  const variableSquare = screen.getByLabelText("14 V1");

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
  fireEvent.click(screen.getByText("△V5"));
  fireEvent.click(screen.getByLabelText("55"));

  expect(
    (screen.getByLabelText("55 V5").firstElementChild as HTMLElement).style
      .transform,
  ).toBe("rotate(180deg)");
});

test("reverses a board variable owner by right click or double tap", () => {
  render(<App />);
  const variableSquare = screen.getByLabelText("14 V1");

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

  expect(attackHand?.textContent).toContain("▲V5");
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

test("starts with help-selfmate and can include helpmate in problem JSON", () => {
  render(<App />);

  const helpmate = screen.getByLabelText("協力詰") as HTMLInputElement;
  const helpSelfmate = screen.getByLabelText("協力自玉詰") as HTMLInputElement;
  expect(helpSelfmate.checked).toBe(true);

  fireEvent.click(helpmate);
  expect(helpmate.checked).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "JSON詳細編集" }));

  expect(
    (screen.getByLabelText("問題JSON") as HTMLTextAreaElement).value,
  ).toContain('"rule": "helpmate"');
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
  const silverSquare = placeSilverAt83();

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

test("copies only formatted solutions", async () => {
  render(<App />);
  fireEvent.click(screen.getByLabelText("協力自玉詰"));
  fireEvent.change(screen.getByLabelText("手数"), {
    target: { value: "4" },
  });
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
          solutions: [["82▲(64)"], ["82▲成(64)"], ["84▲(64)"]],
        }),
      },
    } as MessageEvent);
  });

  expect(await screen.findByText("協力自玉詰 4手")).not.toBeNull();
  expect(screen.getByText(/初形候補世界:/)).not.toBeNull();

  fireEvent.click(await screen.findByRole("button", { name: "解答をコピー" }));

  expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
    [
      "1. 82▲(64) まで 1手",
      "2. 82▲成(64) まで 1手",
      "3. 84▲(64) まで 1手",
    ].join("\n"),
  );
});

test("shows candidates after a clicked solution move", async () => {
  const { container } = render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "検討" }));
  act(() => {
    workerInstances[0].onmessage?.({ data: { type: "ready" } } as MessageEvent);
    workerInstances[0].onmessage?.({
      data: {
        type: "solved",
        requestId: 1,
        responseJson: JSON.stringify({
          worldCount: 2,
          candidates: [{ id: 1, kinds: ["R", "+R"] }],
          solutions: [["84▲(64)", "83玉"]],
          solutionCandidates: [
            [
              [{ id: 1, kinds: ["R"] }],
              [{ id: 1, kinds: ["+R"] }],
            ],
          ],
        }),
      },
    } as MessageEvent);
  });

  expect(await screen.findByText("駒種候補（初形）")).not.toBeNull();
  const table = container.querySelector(".variable-result-table")!;
  expect(within(table as HTMLElement).getByText("飛、龍")).not.toBeNull();

  fireEvent.click(
    screen.getByRole("button", { name: "解1の1手目 84▲(64)" }),
  );
  expect(
    within(table as HTMLElement).getByText("駒種候補（解1：1手目指了図）"),
  ).not.toBeNull();
  expect(within(table as HTMLElement).getByText("飛")).not.toBeNull();

  fireEvent.click(screen.getByRole("button", { name: "解1の2手目 83玉" }));
  expect(
    within(table as HTMLElement).getByText("駒種候補（解1：2手目指了図）"),
  ).not.toBeNull();
  expect(within(table as HTMLElement).getByText("龍")).not.toBeNull();

  fireEvent.click(
    screen.getByRole("button", { name: "駒種候補（初形）に戻す" }),
  );
  expect(within(table as HTMLElement).getByText("駒種候補（初形）")).not.toBeNull();
  expect(within(table as HTMLElement).getByText("飛、龍")).not.toBeNull();
});
