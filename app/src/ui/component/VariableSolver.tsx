import { useState } from "react";
import {
  Alert,
  Button,
  ButtonGroup,
  Card,
  Col,
  Form,
  Row,
  Table,
} from "react-bootstrap";
import { Color, decodeSfen, emptyBoard } from "../../model";
import { solve_variable_problem } from "../../wasm_api";
import Board from "./Board";

const candidateOptions = [
  { code: "P", label: "歩" },
  { code: "L", label: "香" },
  { code: "N", label: "桂" },
  { code: "S", label: "銀" },
  { code: "G", label: "金" },
  { code: "B", label: "角" },
  { code: "R", label: "飛" },
  { code: "K", label: "玉" },
  { code: "+P", label: "と" },
  { code: "+L", label: "杏" },
  { code: "+N", label: "圭" },
  { code: "+S", label: "全" },
  { code: "+B", label: "馬" },
  { code: "+R", label: "龍" },
] as const;

type CandidateCode = (typeof candidateOptions)[number]["code"];
type InputMode = "form" | "json";

interface VariableDraft {
  id: number;
  color: Color;
  square?: string;
  candidates: CandidateCode[];
}

interface ProblemDocument {
  baseSfen: string;
  plies: number;
  variables: Array<{
    id: number;
    color: Color;
    square: string;
    candidates: CandidateCode[];
  }>;
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

const initialBaseSfen = "9/9/kS7/N8/1L7/9/9/9/9 b - 1";
const initialVariables: VariableDraft[] = [
  { id: 1, color: "black", square: "64", candidates: ["R", "+R"] },
];

function buildProblemJson(
  baseSfen: string,
  plies: number,
  variables: VariableDraft[]
): string {
  return JSON.stringify(
    {
      baseSfen,
      plies,
      variables: variables.map((variable) => ({
        id: variable.id,
        color: variable.color,
        square: variable.square ?? "",
        candidates: variable.candidates,
      })),
    },
    null,
    2
  );
}

const initialProblem = buildProblemJson(initialBaseSfen, 1, initialVariables);

export function VariableSolver() {
  const [inputMode, setInputMode] = useState<InputMode>("form");
  const [baseSfen, setBaseSfen] = useState(initialBaseSfen);
  const [plies, setPlies] = useState(1);
  const [variables, setVariables] = useState<VariableDraft[]>(initialVariables);
  const [selectedId, setSelectedId] = useState<number>(1);
  const [manualProblem, setManualProblem] = useState(initialProblem);
  const [maxSolutions, setMaxSolutions] = useState(100);
  const [response, setResponse] = useState<VariableSolveResponse>();
  const [error, setError] = useState<string>();
  const [solving, setSolving] = useState(false);

  const boardResult = (() => {
    try {
      return { board: decodeSfen(baseSfen).board, error: undefined };
    } catch (reason) {
      return {
        board: emptyBoard(),
        error: reason instanceof Error ? reason.message : String(reason),
      };
    }
  })();
  const selected = variables.find((variable) => variable.id === selectedId);
  const generatedProblem = buildProblemJson(baseSfen, plies, variables);

  const clearResult = () => {
    setError(undefined);
    setResponse(undefined);
  };

  const selectMode = (mode: InputMode) => {
    if (mode === "json" && inputMode === "form") {
      setManualProblem(generatedProblem);
    }
    setInputMode(mode);
    clearResult();
  };

  const addVariable = () => {
    const id = variables.reduce((max, variable) => Math.max(max, variable.id), 0) + 1;
    setVariables([...variables, { id, color: "black", candidates: ["P"] }]);
    setSelectedId(id);
    clearResult();
  };

  const removeVariable = (id: number) => {
    const next = variables.filter((variable) => variable.id !== id);
    setVariables(next);
    if (selectedId === id) {
      setSelectedId(next[0]?.id ?? 0);
    }
    clearResult();
  };

  const updateSelected = (change: Partial<VariableDraft>) => {
    setVariables(
      variables.map((variable) =>
        variable.id === selectedId ? { ...variable, ...change } : variable
      )
    );
    clearResult();
  };

  const clickBoard = ([row, col]: [number, number]) => {
    const square = `${col + 1}${row + 1}`;
    const existing = variables.find((variable) => variable.square === square);
    if (existing) {
      setSelectedId(existing.id);
      setError(undefined);
      return;
    }
    if (!selected) {
      setError("先に「覆面駒を追加」を押してください");
      return;
    }
    if (boardResult.board[row][col]) {
      setError(`${square}には通常駒があります。base SFENでは覆面駒のマスを空けてください`);
      return;
    }
    updateSelected({ square });
  };

  const rightClickBoard = ([row, col]: [number, number]) => {
    const square = `${col + 1}${row + 1}`;
    const existing = variables.find((variable) => variable.square === square);
    if (existing) {
      removeVariable(existing.id);
    }
  };

  const toggleCandidate = (candidate: CandidateCode) => {
    if (!selected) {
      return;
    }
    const candidates = selected.candidates.includes(candidate)
      ? selected.candidates.filter((value) => value !== candidate)
      : candidateOptions
          .map((option) => option.code)
          .filter(
            (value) => value === candidate || selected.candidates.includes(value)
          );
    updateSelected({ candidates });
  };

  const loadJsonIntoForm = () => {
    try {
      const value: unknown = JSON.parse(manualProblem);
      if (!isProblemDocument(value)) {
        throw new Error("問題JSONの形式が正しくありません");
      }
      setBaseSfen(value.baseSfen);
      setPlies(value.plies);
      setVariables(value.variables);
      setSelectedId(value.variables[0]?.id ?? 0);
      setInputMode("form");
      clearResult();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const solve = () => {
    setSolving(true);
    clearResult();
    try {
      if (inputMode === "form") {
        if (boardResult.error) {
          throw new Error(`base SFENを解釈できません: ${boardResult.error}`);
        }
        if (variables.length === 0) {
          throw new Error("覆面駒を1枚以上追加してください");
        }
        const unplaced = variables.find((variable) => !variable.square);
        if (unplaced) {
          throw new Error(`V${unplaced.id}を盤面へ配置してください`);
        }
        const emptyCandidates = variables.find(
          (variable) => variable.candidates.length === 0
        );
        if (emptyCandidates) {
          throw new Error(`V${emptyCandidates.id}の候補駒種を選択してください`);
        }
      }
      const json = solve_variable_problem(
        inputMode === "form" ? generatedProblem : manualProblem,
        maxSolutions
      );
      setResponse(JSON.parse(json) as VariableSolveResponse);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSolving(false);
    }
  };

  const selectedPosition = selected?.square
    ? ([
        Number(selected.square[1]) - 1,
        Number(selected.square[0]) - 1,
      ] as [number, number])
    : undefined;

  return (
    <Card className="mb-4">
      <Card.Header as="h2" className="h5 mb-0">
        覆面駒（Variable）検討
      </Card.Header>
      <Card.Body>
        <ButtonGroup className="mb-3" aria-label="問題入力方法">
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

        {inputMode === "form" ? (
          <FormInput
            baseSfen={baseSfen}
            setBaseSfen={(value) => {
              setBaseSfen(value);
              clearResult();
            }}
            plies={plies}
            setPlies={(value) => {
              setPlies(value);
              clearResult();
            }}
            variables={variables}
            selected={selected}
            selectedPosition={selectedPosition}
            setSelectedId={setSelectedId}
            addVariable={addVariable}
            removeVariable={removeVariable}
            updateSelected={updateSelected}
            toggleCandidate={toggleCandidate}
            board={boardResult.board}
            boardError={boardResult.error}
            clickBoard={clickBoard}
            rightClickBoard={rightClickBoard}
            disabled={solving}
          />
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
                disabled={solving}
                spellCheck={false}
              />
            </Form.Group>
            <Button variant="outline-secondary" onClick={loadJsonIntoForm}>
              JSONをフォームに反映
            </Button>
          </div>
        )}

        <div className="d-flex align-items-end gap-3 my-3">
          <Form.Group controlId="variable-max-solutions">
            <Form.Label>最大解数</Form.Label>
            <Form.Control
              type="number"
              min={1}
              max={10000}
              value={maxSolutions}
              onChange={(event) =>
                setMaxSolutions(Math.max(1, Number(event.target.value) || 1))
              }
              disabled={solving}
            />
          </Form.Group>
          <Button onClick={solve} disabled={solving}>
            {solving ? "検討中…" : "覆面駒を検討"}
          </Button>
        </div>

        {error && <Alert variant="danger">{error}</Alert>}
        {response && <VariableResult response={response} />}
      </Card.Body>
    </Card>
  );
}

function FormInput(props: {
  baseSfen: string;
  setBaseSfen: (value: string) => void;
  plies: number;
  setPlies: (value: number) => void;
  variables: VariableDraft[];
  selected?: VariableDraft;
  selectedPosition?: [number, number];
  setSelectedId: (id: number) => void;
  addVariable: () => void;
  removeVariable: (id: number) => void;
  updateSelected: (change: Partial<VariableDraft>) => void;
  toggleCandidate: (candidate: CandidateCode) => void;
  board: ReturnType<typeof emptyBoard>;
  boardError?: string;
  clickBoard: (position: [number, number]) => void;
  rightClickBoard: (position: [number, number]) => void;
  disabled: boolean;
}) {
  return (
    <Row className="g-4">
      <Col lg={6}>
        <Form.Group className="mb-3" controlId="variable-base-sfen">
          <Form.Label>通常駒のbase SFEN</Form.Label>
          <Form.Control
            value={props.baseSfen}
            onChange={(event) => props.setBaseSfen(event.target.value)}
            disabled={props.disabled}
            spellCheck={false}
          />
          <Form.Text>
            覆面駒を置くマスは空けます。受方持駒は標準駒数から補完されます。
          </Form.Text>
        </Form.Group>
        <Form.Group className="mb-3 variable-plies" controlId="variable-plies">
          <Form.Label>手数</Form.Label>
          <Form.Control
            type="number"
            min={1}
            value={props.plies}
            onChange={(event) =>
              props.setPlies(Math.max(1, Number(event.target.value) || 1))
            }
            disabled={props.disabled}
          />
        </Form.Group>
        {props.boardError && (
          <Alert variant="warning">base SFEN: {props.boardError}</Alert>
        )}
        <div className="variable-board-coordinate-shell">
          <div className="variable-file-labels" aria-hidden="true">
            {[9, 8, 7, 6, 5, 4, 3, 2, 1].map((file) => (
              <span key={file}>{file}</span>
            ))}
          </div>
          <div className="d-flex align-items-start">
            <div className="variable-board-wrap">
              <Board
                pieces={props.board}
                selected={props.selectedPosition}
                onClick={props.clickBoard}
                onRightClick={props.rightClickBoard}
                overlay={([row, col]) => {
                  const square = `${col + 1}${row + 1}`;
                  const variable = props.variables.find(
                    (candidate) => candidate.square === square
                  );
                  return variable ? (
                    <div
                      className={`variable-piece variable-piece-${variable.color}${
                        props.selected?.id === variable.id
                          ? " variable-piece-selected"
                          : ""
                      }`}
                    >
                      <span className="variable-owner-mark">
                        {variable.color === "black" ? "▲" : "▽"}
                      </span>
                      V{variable.id}
                    </div>
                  ) : undefined;
                }}
                squareLabel={([row, col]) => {
                  const square = `${col + 1}${row + 1}`;
                  const variable = props.variables.find(
                    (candidate) => candidate.square === square
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
        <Form.Text>
          選択中の覆面駒を置くマスをクリックします。配置済みの覆面駒は右クリックで削除できます。
        </Form.Text>
      </Col>

      <Col lg={6}>
        <div className="d-flex flex-wrap gap-2 mb-3">
          {props.variables.map((variable) => (
            <Button
              key={variable.id}
              size="sm"
              variant={
                props.selected?.id === variable.id
                  ? "primary"
                  : "outline-primary"
              }
              onClick={() => props.setSelectedId(variable.id)}
            >
              V{variable.id} {variable.square ?? "未配置"}
            </Button>
          ))}
          <Button
            size="sm"
            variant="outline-success"
            onClick={props.addVariable}
          >
            ＋ 覆面駒を追加
          </Button>
        </div>

        {props.selected ? (
          <div className="variable-settings border rounded p-3">
            <div className="d-flex justify-content-between align-items-center mb-3">
              <h3 className="h6 mb-0">
                V{props.selected.id}の設定
                {props.selected.square && `（${props.selected.square}）`}
              </h3>
              <Button
                size="sm"
                variant="outline-danger"
                onClick={() => props.removeVariable(props.selected!.id)}
              >
                削除
              </Button>
            </div>
            <fieldset className="mb-3">
              <legend className="form-label fs-6">所属</legend>
              <Form.Check
                inline
                type="radio"
                name={`variable-color-${props.selected.id}`}
                label="先手（▲）"
                checked={props.selected.color === "black"}
                onChange={() => props.updateSelected({ color: "black" })}
              />
              <Form.Check
                inline
                type="radio"
                name={`variable-color-${props.selected.id}`}
                label="後手（▽）"
                checked={props.selected.color === "white"}
                onChange={() => props.updateSelected({ color: "white" })}
              />
            </fieldset>
            <fieldset>
              <legend className="form-label fs-6">候補駒種</legend>
              <div className="variable-candidate-grid">
                {candidateOptions.map((option) => (
                  <Form.Check
                    key={option.code}
                    id={`variable-${props.selected!.id}-${option.code}`}
                    type="checkbox"
                    label={`${option.label}（${option.code}）`}
                    checked={props.selected!.candidates.includes(option.code)}
                    onChange={() => props.toggleCandidate(option.code)}
                  />
                ))}
              </div>
            </fieldset>
          </div>
        ) : (
          <Alert variant="info">「覆面駒を追加」を押してください。</Alert>
        )}
      </Col>
    </Row>
  );
}

function VariableResult(props: { response: VariableSolveResponse }) {
  return (
    <div aria-live="polite">
      <p>
        初形候補世界: <strong>{props.response.worldCount}</strong> ／ 解数: <strong>{props.response.solutions.length}</strong>
      </p>
      <Table bordered size="sm" className="variable-result-table">
        <thead>
          <tr>
            <th>覆面駒</th>
            <th>初形で残る候補</th>
          </tr>
        </thead>
        <tbody>
          {props.response.candidates.map((candidate) => (
            <tr key={candidate.id}>
              <td>V{candidate.id}</td>
              <td>{candidate.kinds.join(", ") || "なし"}</td>
            </tr>
          ))}
        </tbody>
      </Table>
      <h3 className="h6">解答</h3>
      {props.response.solutions.length === 0 ? (
        <p>解なし</p>
      ) : (
        <ol>
          {props.response.solutions.map((solution, index) => (
            <li key={`${index}-${solution.join("-")}`}>
              <code>{solution.join(" ")}</code>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}

function isProblemDocument(value: unknown): value is ProblemDocument {
  if (!value || typeof value !== "object") {
    return false;
  }
  const document = value as Record<string, unknown>;
  return (
    typeof document.baseSfen === "string" &&
    typeof document.plies === "number" &&
    Array.isArray(document.variables) &&
    document.variables.every((variable) => {
      if (!variable || typeof variable !== "object") {
        return false;
      }
      const candidate = variable as Record<string, unknown>;
      return (
        typeof candidate.id === "number" &&
        (candidate.color === "black" || candidate.color === "white") &&
        typeof candidate.square === "string" &&
        Array.isArray(candidate.candidates) &&
        candidate.candidates.every(
          (kind) =>
            typeof kind === "string" &&
            candidateOptions.some((option) => option.code === kind)
        )
      );
    })
  );
}
