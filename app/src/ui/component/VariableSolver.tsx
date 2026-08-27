import { Fragment, useEffect, useReducer, useRef, useState } from "react";
import {
  Alert,
  Button,
  ButtonGroup,
  Card,
  Col,
  Form,
  Row,
  Spinner,
  Table,
} from "react-bootstrap";
import {
  Color,
  Position,
  clonePosition,
  decodeSfen,
  emptyHands,
  encodeSfen,
} from "../../model";
import { positionPieceBox } from "../../model/position";
import { VariableSolverClient } from "../../solve/variable_solver_client";
import { newState, reduce } from "../state/state";
import Board from "./Board";
import Hands from "./Hands";

type InputMode = "form" | "json";
type MateRule = "helpmate" | "helpSelfmate";
type HandVariableMode = "distinguishable" | "indistinguishable";
type VariableLocation = { type: "board"; square: string } | { type: "hand" };

interface VariableDraft {
  id: number;
  color: Color;
  location: VariableLocation;
}

interface ProblemVariable {
  id: number;
  color: Color;
  square?: string;
  inHand?: boolean;
}

interface ProblemDocument {
  baseSfen: string;
  plies: number;
  rule?: MateRule;
  handVariableMode?: HandVariableMode;
  variables: ProblemVariable[];
}

interface VariableCandidates {
  id: number;
  kinds: string[];
}

interface VariableSolveResponse {
  worldCount: number;
  candidates: VariableCandidates[];
  solutions: string[][];
}

type CopyState = "idle" | "copied" | "error";

interface SavedVariableProblem {
  name: string;
  problem: ProblemDocument;
}

interface SavedVariableProblemStore {
  version: 1;
  positions: SavedVariableProblem[];
}

const initialBaseSfen = "9/9/kS7/N8/1L7/9/9/9/9 b - 1";
const singleKingBaseSfen = "4k4/9/9/9/9/9/9/9/9 b - 1";
const doubleKingBaseSfen = "4k4/9/9/9/9/9/9/9/4K4 b - 1";
const initialVariables: VariableDraft[] = [
  {
    id: 1,
    color: "black",
    location: { type: "board", square: "64" },
  },
];
const savedPositionsKey = "hiddenmate_variable_saved_positions";
const maxSavedPositions = 20;
const maxVariables = 6;

function defaultSavedVariableProblems(): SavedVariableProblem[] {
  return [
    {
      name: "単玉のみ",
      problem: buildProblemDocument(singleKingBaseSfen, 1, []),
    },
    {
      name: "双玉のみ",
      problem: buildProblemDocument(doubleKingBaseSfen, 1, []),
    },
  ];
}

function editablePositionFromBaseSfen(sfen: string): Position {
  const position = decodeSfen(sfen);
  position.hands.white = emptyHands();
  position.hands.white = positionPieceBox(position);
  position.hands.white.K = 0;
  return position;
}

function baseSfenFromPosition(position: Position): string {
  const base = clonePosition(position);
  base.hands.white = emptyHands();
  return encodeSfen(base);
}

function buildProblemJson(
  baseSfen: string,
  plies: number,
  rule: MateRule,
  handVariableMode: HandVariableMode,
  variables: VariableDraft[],
): string {
  return JSON.stringify(
    buildProblemDocument(baseSfen, plies, variables, rule, handVariableMode),
    null,
    2,
  );
}

function buildProblemDocument(
  baseSfen: string,
  plies: number,
  variables: VariableDraft[],
  rule: MateRule = "helpmate",
  handVariableMode: HandVariableMode = "indistinguishable",
): ProblemDocument {
  return {
    baseSfen,
    plies,
    rule,
    handVariableMode,
    variables: variables.map((variable) => ({
      id: variable.id,
      color: variable.color,
      ...(variable.location.type === "board"
        ? { square: variable.location.square }
        : { inHand: true }),
    })),
  };
}

function loadSavedVariableProblems(): SavedVariableProblem[] {
  try {
    const raw = localStorage.getItem(savedPositionsKey);
    if (!raw) {
      return defaultSavedVariableProblems();
    }
    const parsed: unknown = JSON.parse(raw);
    if (
      parsed &&
      typeof parsed === "object" &&
      (parsed as Record<string, unknown>).version === 1
    ) {
      return validSavedVariableProblems(
        (parsed as Record<string, unknown>).positions,
      ).slice(0, maxSavedPositions);
    }

    // 旧形式（配列）から移行する際だけ、デフォルト2局面を補う。
    const legacy = validSavedVariableProblems(parsed);
    const legacyNames = new Set(legacy.map((position) => position.name));
    return [
      ...defaultSavedVariableProblems().filter(
        (position) => !legacyNames.has(position.name),
      ),
      ...legacy,
    ].slice(0, maxSavedPositions);
  } catch {
    return defaultSavedVariableProblems();
  }
}

function validSavedVariableProblems(value: unknown): SavedVariableProblem[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((entry): entry is SavedVariableProblem => {
    if (!entry || typeof entry !== "object") {
      return false;
    }
    const candidate = entry as Record<string, unknown>;
    return (
      typeof candidate.name === "string" && isProblemDocument(candidate.problem)
    );
  });
}

const initialProblem = buildProblemJson(
  initialBaseSfen,
  1,
  "helpmate",
  "indistinguishable",
  initialVariables,
);

export function VariableSolver() {
  const [editorState, dispatch] = useReducer(reduce, undefined, () => {
    const state = newState();
    state.position = editablePositionFromBaseSfen(initialBaseSfen);
    return state;
  });
  const [inputMode, setInputMode] = useState<InputMode>("form");
  const [sfenInput, setSfenInput] = useState(initialBaseSfen);
  const [plies, setPlies] = useState(1);
  const [rule, setRule] = useState<MateRule>("helpmate");
  const [handVariableMode, setHandVariableMode] =
    useState<HandVariableMode>("indistinguishable");
  const [variables, setVariables] = useState<VariableDraft[]>(initialVariables);
  const [selectedId, setSelectedId] = useState<number>(0);
  const [manualProblem, setManualProblem] = useState(initialProblem);
  const [maxSolutions, setMaxSolutions] = useState(100);
  const [response, setResponse] = useState<VariableSolveResponse>();
  const [error, setError] = useState<string>();
  const [solving, setSolving] = useState(false);
  const solverClient = useRef<VariableSolverClient>();
  const [savedPositions, setSavedPositions] = useState<SavedVariableProblem[]>(
    loadSavedVariableProblems,
  );

  const baseSfen = baseSfenFromPosition(editorState.position);
  const selected = variables.find((variable) => variable.id === selectedId);
  const generatedProblem = buildProblemJson(
    baseSfen,
    plies,
    rule,
    handVariableMode,
    variables,
  );

  useEffect(() => setSfenInput(baseSfen), [baseSfen]);
  useEffect(
    () => () => {
      solverClient.current?.dispose();
    },
    [],
  );
  useEffect(() => {
    try {
      const store: SavedVariableProblemStore = {
        version: 1,
        positions: savedPositions,
      };
      localStorage.setItem(savedPositionsKey, JSON.stringify(store));
    } catch {
      // 保存容量不足などの場合も、盤面編集と検討は継続できるようにする。
    }
  }, [savedPositions]);

  const clearResult = () => {
    if (solving) {
      solverClient.current?.cancel();
      setSolving(false);
    }
    setError(undefined);
    setResponse(undefined);
  };

  const changeHandVariableMode = (mode: HandVariableMode) => {
    setHandVariableMode(mode);
    clearResult();
  };

  const selectVariable = (id: number) => {
    const nextId = id === selectedId ? 0 : id;
    setSelectedId(nextId);
    if (nextId !== 0) {
      dispatch({ ty: "clear-selection" });
    }
  };

  const selectMode = (mode: InputMode) => {
    if (mode === "json" && inputMode === "form") {
      setManualProblem(generatedProblem);
    }
    setInputMode(mode);
    clearResult();
  };

  const addVariableToHand = (color: Color) => {
    if (variables.length >= maxVariables) {
      return;
    }
    const id =
      variables.reduce((max, variable) => Math.max(max, variable.id), 0) + 1;
    setVariables([...variables, { id, color, location: { type: "hand" } }]);
    setSelectedId(0);
    clearResult();
  };

  const removeVariable = (id: number) => {
    const next = variables.filter((variable) => variable.id !== id);
    setVariables(next);
    if (selectedId === id) {
      setSelectedId(0);
    }
    clearResult();
  };

  const updateSelected = (change: Partial<VariableDraft>) => {
    setVariables(
      variables.map((variable) =>
        variable.id === selectedId ? { ...variable, ...change } : variable,
      ),
    );
    clearResult();
  };

  const variableAt = (square: string) =>
    variables.find(
      (variable) =>
        variable.location.type === "board" &&
        variable.location.square === square,
    );

  const clickBoard = ([row, col]: [number, number]) => {
    const square = `${col + 1}${row + 1}`;
    const existing = variableAt(square);
    if (existing) {
      selectVariable(existing.id);
      setError(undefined);
      return;
    }
    if (selected) {
      if (editorState.position.board[row][col]) {
        setSelectedId(0);
        dispatch({ ty: "click-board", pos: [row, col] });
        clearResult();
        return;
      }
      updateSelected({ location: { type: "board", square } });
      setSelectedId(0);
      return;
    }
    dispatch({ ty: "click-board", pos: [row, col] });
    clearResult();
  };

  const rightClickBoard = ([row, col]: [number, number]) => {
    const square = `${col + 1}${row + 1}`;
    const existing = variableAt(square);
    if (existing) {
      setVariables(
        variables.map((variable) =>
          variable.id === existing.id
            ? {
                ...variable,
                color: variable.color === "black" ? "white" : "black",
              }
            : variable,
        ),
      );
      setSelectedId(0);
      dispatch({ ty: "clear-selection" });
      clearResult();
      return;
    }
    dispatch({ ty: "right-click-board", pos: [row, col] });
    clearResult();
  };

  const clickKnownHand = (
    color: Color,
    kind: Parameters<typeof Hands>[0]["selected"],
  ) => {
    setSelectedId(0);
    dispatch({
      ty: "click-hand",
      color,
      kind: kind === "" ? undefined : kind,
    });
    clearResult();
  };

  const moveSelectedToHand = (color: Color) => {
    if (!selected) {
      return;
    }
    updateSelected({ color, location: { type: "hand" } });
    setSelectedId(0);
  };

  const loadSfen = () => {
    try {
      dispatch({
        ty: "set-position",
        position: editablePositionFromBaseSfen(sfenInput),
      });
      clearResult();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const applyProblemToForm = (problem: ProblemDocument) => {
    dispatch({
      ty: "set-position",
      position: editablePositionFromBaseSfen(problem.baseSfen),
    });
    setPlies(problem.plies);
    setRule(problem.rule ?? "helpmate");
    setHandVariableMode(problem.handVariableMode ?? "indistinguishable");
    const loaded = problem.variables.map(problemVariableToDraft);
    setVariables(loaded);
    setSelectedId(0);
    setInputMode("form");
    clearResult();
  };

  const loadJsonIntoForm = () => {
    try {
      const value: unknown = JSON.parse(manualProblem);
      if (!isProblemDocument(value)) {
        throw new Error("問題JSONの形式が正しくありません");
      }
      applyProblemToForm(value);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const saveCurrentPosition = (name: string) => {
    const trimmed = name.trim() || baseSfen;
    const saved = {
      name: trimmed,
      problem: buildProblemDocument(
        baseSfen,
        plies,
        variables,
        rule,
        handVariableMode,
      ),
    };
    setSavedPositions((current) =>
      [saved, ...current].slice(0, maxSavedPositions),
    );
  };

  const deleteSavedPosition = (index: number) => {
    setSavedPositions((current) => current.filter((_, i) => i !== index));
  };

  const solve = async () => {
    if (solving) {
      solverClient.current?.cancel();
      setSolving(false);
      return;
    }

    if (inputMode === "form" && variables.length === 0) {
      setError("覆面駒を1枚以上追加してください");
      setResponse(undefined);
      return;
    }

    setSolving(true);
    clearResult();
    try {
      const client =
        solverClient.current ??
        (solverClient.current = new VariableSolverClient());
      const json = await client.solve(
        inputMode === "form" ? generatedProblem : manualProblem,
        maxSolutions,
      );
      if (json !== undefined) {
        setResponse(JSON.parse(json) as VariableSolveResponse);
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSolving(false);
    }
  };

  return (
    <Card className="mb-4">
      <Card.Header as="h2" className="h5 mb-0">
        覆面駒
      </Card.Header>
      <Card.Body>
        <div className="d-flex flex-wrap align-items-center gap-2 mb-3">
          <ButtonGroup aria-label="問題入力方法">
            <Button
              variant={inputMode === "form" ? "primary" : "outline-primary"}
              onClick={() => selectMode("form")}
            >
              盤面・フォーム
            </Button>
            <Button
              variant={inputMode === "json" ? "primary" : "outline-primary"}
              onClick={() => selectMode("json")}
            >
              JSON詳細編集
            </Button>
          </ButtonGroup>
        </div>

        {inputMode === "form" ? (
          <>
            <Form.Group className="mb-3" controlId="variable-base-sfen">
              <Form.Label>通常駒のbase SFEN</Form.Label>
              <div className="d-flex gap-2">
                <Form.Control
                  className="variable-sfen-input"
                  size="sm"
                  value={sfenInput}
                  onChange={(event) => setSfenInput(event.target.value)}
                  spellCheck={false}
                />
                <Button
                  className="text-nowrap"
                  size="sm"
                  variant="outline-secondary"
                  onClick={loadSfen}
                >
                  読込
                </Button>
              </div>
            </Form.Group>
            <Row className="g-4 variable-three-column-layout">
              <Col xl={4} className="variable-layout-board">
                <VariablePositionEditor
                  position={editorState.position}
                  normalSelected={editorState.selected}
                  variables={variables}
                  selected={selected}
                  clickBoard={clickBoard}
                  rightClickBoard={rightClickBoard}
                  clickKnownHand={clickKnownHand}
                  moveSelectedToHand={moveSelectedToHand}
                  selectVariable={selectVariable}
                />
              </Col>
              <Col xl={4} className="variable-layout-settings">
                <VariableSavedPositions
                  positions={savedPositions}
                  defaultName={baseSfen}
                  disabled={solving}
                  onSave={saveCurrentPosition}
                  onLoad={(position) => applyProblemToForm(position.problem)}
                  onDelete={deleteSavedPosition}
                />
                <VariableControls
                  variables={variables}
                  selected={selected}
                  setSelectedId={selectVariable}
                  removeVariable={removeVariable}
                  addVariableToHand={addVariableToHand}
                />
              </Col>
              <Col xl={4} className="variable-layout-results">
                <VariableSolveControls
                  plies={plies}
                  setPlies={setPlies}
                  rule={rule}
                  setRule={setRule}
                  handVariableMode={handVariableMode}
                  setHandVariableMode={changeHandVariableMode}
                  maxSolutions={maxSolutions}
                  setMaxSolutions={setMaxSolutions}
                  solving={solving}
                  onSolve={solve}
                />
                {error && <Alert variant="danger">{error}</Alert>}
                {response && <VariableResult response={response} />}
              </Col>
            </Row>
          </>
        ) : (
          <div>
            <Form.Group className="mb-2" controlId="variable-problem-json">
              <Form.Label>問題JSON</Form.Label>
              <Form.Control
                as="textarea"
                className="variable-problem-json"
                value={manualProblem}
                onChange={(event) => {
                  setManualProblem(event.target.value);
                  clearResult();
                }}
                spellCheck={false}
              />
            </Form.Group>
            <div className="d-flex flex-wrap gap-2">
              <Button variant="outline-secondary" onClick={loadJsonIntoForm}>
                JSONをフォームに反映
              </Button>
              <CopyButton text={manualProblem} idleLabel="問題JSONをコピー" />
            </div>
          </div>
        )}

        {inputMode === "json" && (
          <div className="my-3">
            <VariableSolveControls
              maxSolutions={maxSolutions}
              setMaxSolutions={setMaxSolutions}
              solving={solving}
              onSolve={solve}
            />
            {error && <Alert variant="danger">{error}</Alert>}
            {response && <VariableResult response={response} />}
          </div>
        )}
      </Card.Body>
    </Card>
  );
}

function VariableSavedPositions(props: {
  positions: SavedVariableProblem[];
  defaultName: string;
  disabled: boolean;
  onSave: (name: string) => void;
  onLoad: (position: SavedVariableProblem) => void;
  onDelete: (index: number) => void;
}) {
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const startAdding = () => {
    setName(props.defaultName);
    setAdding(true);
    setTimeout(() => inputRef.current?.select(), 0);
  };
  const confirmAdd = () => {
    props.onSave(name);
    setAdding(false);
  };

  return (
    <div className="variable-saved-positions border rounded p-3 mb-3">
      <div className="d-flex align-items-center gap-2 mb-2">
        <h3 className="h6 mb-0">保存局面</h3>
        {!adding && (
          <Button
            aria-label="現在の局面を保存"
            size="sm"
            variant="secondary"
            onClick={startAdding}
            disabled={props.disabled}
          >
            ＋
          </Button>
        )}
      </div>
      {adding && (
        <div className="d-flex gap-1 mb-2">
          <Form.Control
            ref={inputRef}
            aria-label="保存名"
            size="sm"
            value={name}
            onChange={(event) => setName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") confirmAdd();
              if (event.key === "Escape") setAdding(false);
            }}
          />
          <Button size="sm" onClick={confirmAdd}>
            保存
          </Button>
          <Button
            aria-label="保存をキャンセル"
            size="sm"
            variant="outline-secondary"
            onClick={() => setAdding(false)}
          >
            ×
          </Button>
        </div>
      )}
      {props.positions.length === 0 ? (
        <div className="small text-muted">保存された局面はありません。</div>
      ) : (
        <div className="d-grid gap-1">
          {props.positions.map((position, index) => (
            <div className="d-flex gap-1" key={`${index}-${position.name}`}>
              <Button
                className="flex-grow-1 text-start text-truncate"
                size="sm"
                variant="outline-secondary"
                title={position.name}
                onClick={() => props.onLoad(position)}
                disabled={props.disabled}
              >
                {position.name}
              </Button>
              <Button
                aria-label={`${position.name}を削除`}
                size="sm"
                variant="outline-danger"
                onClick={() => props.onDelete(index)}
                disabled={props.disabled}
              >
                ×
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function VariablePositionEditor(props: {
  position: Position;
  normalSelected: ReturnType<typeof newState>["selected"];
  variables: VariableDraft[];
  selected?: VariableDraft;
  clickBoard: (position: [number, number]) => void;
  rightClickBoard: (position: [number, number]) => void;
  clickKnownHand: (
    color: Color,
    kind: Parameters<typeof Hands>[0]["selected"],
  ) => void;
  moveSelectedToHand: (color: Color) => void;
  selectVariable: (id: number) => void;
}) {
  const boardSelected =
    props.selected?.location.type === "board"
      ? squareToPosition(props.selected.location.square)
      : props.normalSelected.shown && props.normalSelected.ty === "board"
        ? props.normalSelected.pos
        : undefined;
  const selectedHand = (color: Color) =>
    props.normalSelected.shown &&
    props.normalSelected.ty === "hand" &&
    props.normalSelected.color === color
      ? (props.normalSelected.kind ?? "")
      : undefined;
  const clickHand = (
    color: Color,
    kind: Parameters<typeof Hands>[0]["selected"],
  ) => {
    if (props.selected) {
      props.moveSelectedToHand(color);
      return;
    }
    props.clickKnownHand(color, kind);
  };

  return (
    <div className="variable-position-editor">
      <VariableHand
        color="white"
        hands={props.position.hands.white}
        selectedKind={selectedHand("white")}
        variables={props.variables}
        onKnownClick={(kind) => clickHand("white", kind)}
        onHandClick={() => clickHand("white", undefined)}
        variableSelected={props.selected !== undefined}
        selectedId={props.selected?.id}
        onVariableClick={props.selectVariable}
      />
      <CoordinateBoard
        position={props.position}
        variables={props.variables}
        selectedId={props.selected?.id}
        selectedPosition={boardSelected}
        onClick={props.clickBoard}
        onRightClick={props.rightClickBoard}
      />
      <VariableHand
        color="black"
        hands={props.position.hands.black}
        selectedKind={selectedHand("black")}
        variables={props.variables}
        onKnownClick={(kind) => clickHand("black", kind)}
        onHandClick={() => clickHand("black", undefined)}
        variableSelected={props.selected !== undefined}
        selectedId={props.selected?.id}
        onVariableClick={props.selectVariable}
      />
    </div>
  );
}

function VariableHand(props: {
  color: Color;
  hands: Position["hands"][Color];
  selectedKind: Parameters<typeof Hands>[0]["selected"];
  variables: VariableDraft[];
  onKnownClick: Parameters<typeof Hands>[0]["onClick"];
  onHandClick: () => void;
  variableSelected: boolean;
  selectedId?: number;
  onVariableClick: (id: number) => void;
}) {
  const symbol = props.color === "black" ? "▲" : "△";
  const destination = props.color === "black" ? "攻方駒台" : "受方駒台";
  const variables = props.variables.filter(
    (variable) =>
      variable.color === props.color && variable.location.type === "hand",
  );
  return (
    <div
      className={`variable-hand variable-hand-${props.color}`}
      onClick={props.onHandClick}
      title={
        props.variableSelected
          ? `選択中の覆面駒を${destination}へ移動`
          : undefined
      }
    >
      <Hands
        hands={props.hands}
        selected={props.selectedKind}
        onClick={props.onKnownClick}
        showNothing={variables.length === 0}
      />
      {variables.length > 0 && (
        <div className="d-flex flex-wrap align-items-center gap-2 mt-1">
          {variables.map((variable) => (
            <span
              key={variable.id}
              className={`badge variable-hand-piece variable-hand-piece-${props.color}${
                props.selectedId === variable.id
                  ? " variable-hand-piece-selected"
                  : ""
              }`}
              onClick={(event) => {
                event.stopPropagation();
                props.onVariableClick(variable.id);
              }}
            >
              {symbol}V{variable.id}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

function CoordinateBoard(props: {
  position: Position;
  variables: VariableDraft[];
  selectedId?: number;
  selectedPosition?: [number, number];
  onClick: (position: [number, number]) => void;
  onRightClick: (position: [number, number]) => void;
}) {
  return (
    <div className="variable-board-coordinate-shell my-2">
      <div className="variable-file-labels" aria-hidden="true">
        {[9, 8, 7, 6, 5, 4, 3, 2, 1].map((file) => (
          <span key={file}>{file}</span>
        ))}
      </div>
      <div className="d-flex align-items-start">
        <div className="variable-board-wrap">
          <Board
            pieces={props.position.board}
            selected={props.selectedPosition}
            onClick={props.onClick}
            onRightClick={props.onRightClick}
            overlay={([row, col]) => {
              const square = `${col + 1}${row + 1}`;
              const variable = props.variables.find(
                (candidate) =>
                  candidate.location.type === "board" &&
                  candidate.location.square === square,
              );
              return variable ? (
                <div
                  className={`variable-piece variable-piece-${variable.color}${
                    props.selectedId === variable.id
                      ? " variable-piece-selected"
                      : ""
                  }`}
                  style={{
                    transform:
                      variable.color === "white"
                        ? "rotate(180deg)"
                        : "rotate(0deg)",
                  }}
                >
                  <span className="variable-owner-mark">
                    {variable.color === "black" ? "▲" : "△"}
                  </span>
                  V{variable.id}
                </div>
              ) : undefined;
            }}
            squareLabel={([row, col]) => {
              const square = `${col + 1}${row + 1}`;
              const variable = props.variables.find(
                (candidate) =>
                  candidate.location.type === "board" &&
                  candidate.location.square === square,
              );
              return variable ? `${square} V${variable.id}` : square;
            }}
          />
        </div>
        <div className="variable-rank-labels" aria-hidden="true">
          {[1, 2, 3, 4, 5, 6, 7, 8, 9].map((rank) => (
            <span key={rank}>{rank}</span>
          ))}
        </div>
      </div>
    </div>
  );
}

function VariableControls(props: {
  variables: VariableDraft[];
  selected?: VariableDraft;
  setSelectedId: (id: number) => void;
  removeVariable: (id: number) => void;
  addVariableToHand: (color: Color) => void;
}) {
  return (
    <div className="variable-control-panel border rounded p-3">
      <h3 className="h6">覆面駒の新規追加</h3>
      <div className="variable-add-buttons d-flex gap-2 mb-3">
        <Button
          className="flex-fill text-nowrap"
          size="sm"
          variant="outline-primary"
          disabled={props.variables.length >= maxVariables}
          onClick={() => props.addVariableToHand("black")}
        >
          攻方持駒に追加
        </Button>
        <Button
          className="flex-fill text-nowrap"
          size="sm"
          variant="outline-primary"
          disabled={props.variables.length >= maxVariables}
          onClick={() => props.addVariableToHand("white")}
        >
          受方持駒に追加
        </Button>
      </div>
      <h3 className="h6">覆面駒一覧</h3>
      {props.variables.length > 0 ? (
        <div className="d-flex flex-wrap gap-2">
          {props.variables.map((variable) => {
            const symbol = variable.color === "black" ? "▲" : "△";
            return (
              <ButtonGroup key={variable.id}>
                <Button
                  size="sm"
                  variant={
                    props.selected?.id === variable.id
                      ? "primary"
                      : "outline-primary"
                  }
                  onClick={() =>
                    props.setSelectedId(
                      props.selected?.id === variable.id ? 0 : variable.id,
                    )
                  }
                >
                  {symbol}V{variable.id} {locationLabel(variable.location)}
                </Button>
                <Button
                  aria-label={`V${variable.id}を削除`}
                  title={`V${variable.id}を削除`}
                  size="sm"
                  variant="outline-danger"
                  onClick={() => props.removeVariable(variable.id)}
                >
                  ×
                </Button>
              </ButtonGroup>
            );
          })}
        </div>
      ) : (
        <Alert variant="info" className="mb-0">
          覆面駒はまだありません。
        </Alert>
      )}
    </div>
  );
}

function VariableSolveControls(props: {
  plies?: number;
  setPlies?: (plies: number) => void;
  rule?: MateRule;
  setRule?: (rule: MateRule) => void;
  handVariableMode?: HandVariableMode;
  setHandVariableMode?: (mode: HandVariableMode) => void;
  maxSolutions: number;
  setMaxSolutions: (maxSolutions: number) => void;
  solving: boolean;
  onSolve: () => void;
}) {
  const [pliesInput, setPliesInput] = useState(
    props.plies === undefined ? "" : String(props.plies),
  );

  useEffect(() => {
    if (props.plies !== undefined) {
      setPliesInput(String(props.plies));
    }
  }, [props.plies]);

  return (
    <div className="variable-solve-controls mb-3">
      {props.rule !== undefined && props.setRule && (
        <Form.Group
          className="variable-solve-group variable-rule"
          controlId="variable-rule"
        >
          <Form.Label>ルール</Form.Label>
          <div className="d-flex gap-2 text-nowrap">
            <Form.Check
              inline
              type="radio"
              name="variable-rule"
              id="variable-rule-helpmate"
              label="協力詰"
              checked={props.rule === "helpmate"}
              onChange={() => props.setRule!("helpmate")}
              disabled={props.solving}
            />
            <Form.Check
              inline
              type="radio"
              name="variable-rule"
              id="variable-rule-help-selfmate"
              label="協力自玉詰"
              checked={props.rule === "helpSelfmate"}
              onChange={() => props.setRule!("helpSelfmate")}
              disabled={props.solving}
            />
          </div>
        </Form.Group>
      )}
      {props.handVariableMode !== undefined && props.setHandVariableMode && (
        <Form.Group
          className="variable-solve-group variable-hand-mode"
          controlId="variable-hand-mode"
        >
          <Form.Label>駒台の覆面駒</Form.Label>
          <div className="d-flex gap-2 text-nowrap">
            <Form.Check
              inline
              type="radio"
              name="variable-hand-mode"
              id="variable-hand-mode-indistinguishable"
              label="区別しない"
              checked={props.handVariableMode === "indistinguishable"}
              onChange={() => props.setHandVariableMode!("indistinguishable")}
              disabled={props.solving}
            />
            <Form.Check
              inline
              type="radio"
              name="variable-hand-mode"
              id="variable-hand-mode-distinguishable"
              label="区別する"
              checked={props.handVariableMode === "distinguishable"}
              onChange={() => props.setHandVariableMode!("distinguishable")}
              disabled={props.solving}
            />
          </div>
        </Form.Group>
      )}
      <div className="variable-solve-group variable-solve-actions d-flex flex-wrap align-items-end gap-2">
        {props.plies !== undefined && props.setPlies && (
          <Form.Group
            className="variable-control-number"
            controlId="variable-plies"
          >
            <Form.Label>手数</Form.Label>
            <Form.Control
              size="sm"
              type="number"
              min={1}
              value={pliesInput}
              onChange={(event) => {
                const value = event.target.value;
                setPliesInput(value);
                if (value !== "") {
                  props.setPlies!(Math.max(1, Number(value) || 1));
                }
              }}
              onBlur={() => {
                if (pliesInput === "") {
                  setPliesInput("1");
                  props.setPlies!(1);
                }
              }}
              disabled={props.solving}
            />
          </Form.Group>
        )}
        <Form.Group
          className="variable-control-number"
          controlId="variable-max-solutions"
        >
          <Form.Label>最大解数</Form.Label>
          <Form.Control
            size="sm"
            type="number"
            min={1}
            max={10000}
            value={props.maxSolutions}
            onChange={(event) =>
              props.setMaxSolutions(
                Math.max(1, Number(event.target.value) || 1),
              )
            }
            disabled={props.solving}
          />
        </Form.Group>
        <Button
          className="variable-solve-button text-nowrap"
          size="sm"
          onClick={props.onSolve}
          variant={props.solving ? "danger" : "primary"}
        >
          {props.solving ? "中断" : "検討"}
        </Button>
        {props.solving && (
          <Spinner animation="border" role="status">
            <span className="visually-hidden">検討中...</span>
          </Spinner>
        )}
      </div>
    </div>
  );
}

function VariableResult(props: { response: VariableSolveResponse }) {
  const copyText = formatResultForCopy(props.response);
  return (
    <div aria-live="polite">
      <p>
        初形候補世界: <strong>{props.response.worldCount}</strong> ／ 解数:{" "}
        <strong>{props.response.solutions.length}</strong>
      </p>
      <Table bordered size="sm" className="variable-result-table">
        <thead>
          <tr>
            <th>覆面駒</th>
            <th>初形での駒種候補</th>
          </tr>
        </thead>
        <tbody>
          {props.response.candidates.map((candidate) => (
            <tr key={candidate.id}>
              <td>V{candidate.id}</td>
              <td>
                {candidate.kinds.map(japaneseCandidateKind).join("、") ||
                  "なし"}
              </td>
            </tr>
          ))}
        </tbody>
      </Table>
      <div className="d-flex flex-wrap align-items-center gap-2 mb-2">
        <h3 className="h6 mb-0">解答</h3>
        <CopyButton text={copyText} idleLabel="解答をコピー" />
      </div>
      {props.response.solutions.length === 0 ? (
        <p>解なし</p>
      ) : (
        <ol className="variable-solution-list">
          {props.response.solutions.map((solution, index) => (
            <li key={`${index}-${solution.join("-")}`}>
              <code>
                {solution.map((move, moveIndex) => (
                  <Fragment key={`${moveIndex}-${move}`}>
                    {move}
                    {moveIndex < solution.length - 1 && (
                      <>
                        {" "}
                        {(moveIndex + 1) % 4 === 0 && (
                          <br className="variable-solution-break-sp" />
                        )}
                        {(moveIndex + 1) % 6 === 0 && (
                          <br className="variable-solution-break-pc" />
                        )}
                      </>
                    )}
                  </Fragment>
                ))}{" "}
                まで {solution.length}手
              </code>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}

function CopyButton(props: { text: string; idleLabel: string }) {
  const [state, setState] = useState<CopyState>("idle");

  useEffect(() => setState("idle"), [props.text]);

  const copy = async () => {
    try {
      await copyToClipboard(props.text);
      setState("copied");
    } catch {
      setState("error");
    }
  };

  return (
    <Button size="sm" variant="outline-secondary" onClick={copy}>
      <span aria-live="polite">
        {state === "copied"
          ? "コピーしました"
          : state === "error"
            ? "コピーできませんでした"
            : props.idleLabel}
      </span>
    </Button>
  );
}

async function copyToClipboard(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.select();
  const copied = document.execCommand?.("copy") ?? false;
  textarea.remove();
  if (!copied) {
    throw new Error("clipboard is unavailable");
  }
}

function formatResultForCopy(response: VariableSolveResponse): string {
  const candidates = response.candidates.map(
    (candidate) =>
      `V${candidate.id}: ${candidate.kinds.map(japaneseCandidateKind).join("、") || "なし"}`,
  );
  const solutions =
    response.solutions.length === 0
      ? ["解なし"]
      : response.solutions.map(
          (solution, index) =>
            `${index + 1}. ${solution.join(" ")} まで ${solution.length}手`,
        );
  return [
    `初形候補世界: ${response.worldCount}`,
    `解数: ${response.solutions.length}`,
    "",
    "初形での駒種候補",
    ...candidates,
    "",
    "解答",
    ...solutions,
  ].join("\n");
}

function japaneseCandidateKind(kind: string): string {
  const names: Record<string, string> = {
    P: "歩",
    L: "香",
    N: "桂",
    S: "銀",
    G: "金",
    B: "角",
    R: "飛",
    K: "玉",
    "+P": "と",
    "+L": "杏",
    "+N": "圭",
    "+S": "全",
    "+B": "馬",
    "+R": "龍",
  };
  return names[kind] ?? kind;
}

function problemVariableToDraft(variable: ProblemVariable): VariableDraft {
  if (variable.inHand) {
    return {
      id: variable.id,
      color: variable.color,
      location: { type: "hand" },
    };
  }
  if (variable.square) {
    return {
      id: variable.id,
      color: variable.color,
      location: { type: "board", square: variable.square },
    };
  }
  throw new Error(`V${variable.id}の配置場所がありません`);
}

function isProblemDocument(value: unknown): value is ProblemDocument {
  if (!value || typeof value !== "object") {
    return false;
  }
  const document = value as Record<string, unknown>;
  return (
    typeof document.baseSfen === "string" &&
    typeof document.plies === "number" &&
    (document.rule === undefined ||
      document.rule === "helpmate" ||
      document.rule === "helpSelfmate") &&
    (document.handVariableMode === undefined ||
      document.handVariableMode === "distinguishable" ||
      document.handVariableMode === "indistinguishable") &&
    Array.isArray(document.variables) &&
    document.variables.every((variable) => {
      if (!variable || typeof variable !== "object") {
        return false;
      }
      const candidate = variable as Record<string, unknown>;
      const hasLocation =
        typeof candidate.square === "string" || candidate.inHand === true;
      return (
        typeof candidate.id === "number" &&
        (candidate.color === "black" || candidate.color === "white") &&
        hasLocation
      );
    })
  );
}

function locationLabel(location: VariableLocation): string {
  return location.type === "board" ? location.square : "駒台";
}

function squareToPosition(square: string): [number, number] {
  return [Number(square[1]) - 1, Number(square[0]) - 1];
}
