import { useEffect, useReducer, useRef, useState } from "react";
import { Alert, Button, ButtonGroup, Card, Col, Form, Row } from "react-bootstrap";
import { BsChevronDown, BsChevronUp } from "react-icons/bs";
import { Color, Kind, decodeSfen, encodeSfen } from "../../model";
import { KnownInvisibleSolverClient } from "../../solve/known_invisible_solver_client";
import { newState, reduce } from "../state/state";
import { ShiftDirection } from "./Shifter";
import { VariablePositionEditor, VariableSolveControls } from "./VariableSolver";

type MateRule = "helpmate" | "helpSelfmate" | "bestMate";
type InputMode = "form" | "json";
type InvisibleKind = Kind | "+P" | "+L" | "+N" | "+S" | "+B" | "+R";
type Counts = Record<Color, Record<InvisibleKind, number>>;

const kinds: Array<[InvisibleKind, string]> = [
  ["K", "玉"], ["R", "飛"], ["B", "角"], ["G", "金"], ["S", "銀"],
  ["N", "桂"], ["L", "香"], ["P", "歩"], ["+R", "龍"], ["+B", "馬"],
  ["+S", "全"], ["+N", "圭"], ["+L", "杏"], ["+P", "と"],
];
const initialSfen = "4k4/9/9/9/9/9/9/9/4K4 b 2r2b4g4s4n4l18p 1";

interface ProblemDocument {
  baseSfen: string;
  plies: number;
  rule?: MateRule;
  invisibles: Array<{ color: Color; kind: InvisibleKind; count: number }>;
}

interface SolveResponse {
  worldCount: number;
  solutions: string[][];
}

function formatInvisibleSummary(problem: ProblemDocument): string {
  const counts = emptyCounts();
  for (const spec of problem.invisibles) {
    const kind = spec.kind.toUpperCase() as InvisibleKind;
    if (kinds.some(([candidate]) => candidate === kind)) {
      counts[spec.color][kind] += spec.count;
    }
  }
  const entries = (["black", "white"] as Color[]).flatMap((color) =>
    kinds
      .filter(([kind]) => counts[color][kind] > 0)
      .map(([kind, label]) =>
        `${color === "black" ? "攻方" : "受方"}透明${label}${counts[color][kind]}枚`,
      ),
  );
  return entries.join("、") || "透明駒なし";
}

function emptyCounts(): Counts {
  const side = () => Object.fromEntries(kinds.map(([kind]) => [kind, 0])) as Record<InvisibleKind, number>;
  return { black: side(), white: side() };
}

function buildProblemJson(positionSfen: string, plies: number, rule: MateRule, counts: Counts) {
  return JSON.stringify({
    baseSfen: positionSfen,
    plies,
    rule,
    invisibles: (["white", "black"] as Color[]).flatMap((color) =>
      kinds.filter(([kind]) => counts[color][kind] > 0)
        .map(([kind]) => ({ color, kind, count: counts[color][kind] })),
    ),
  }, null, 2);
}

export function KnownInvisibleSolver() {
  const [open, setOpen] = useState(false);
  const [inputMode, setInputMode] = useState<InputMode>("form");
  const [editorState, dispatch] = useReducer(reduce, undefined, () => {
    const state = newState();
    state.position = decodeSfen(initialSfen);
    return state;
  });
  const [sfenInput, setSfenInput] = useState(initialSfen);
  const [counts, setCounts] = useState<Counts>(emptyCounts);
  const [draftColor, setDraftColor] = useState<Color>("black");
  const [draftKind, setDraftKind] = useState<InvisibleKind>("P");
  const [plies, setPlies] = useState(3);
  const [rule, setRule] = useState<MateRule>("helpmate");
  const [maxSolutions, setMaxSolutions] = useState(20);
  const [manualProblem, setManualProblem] = useState(buildProblemJson(initialSfen, 3, "helpmate", emptyCounts()));
  const [solving, setSolving] = useState(false);
  const [response, setResponse] = useState<SolveResponse>();
  const [solvedProblem, setSolvedProblem] = useState<ProblemDocument>();
  const [error, setError] = useState<string>();
  const client = useRef<KnownInvisibleSolverClient>();

  useEffect(() => () => client.current?.dispose(), []);
  useEffect(() => setSfenInput(encodeSfen(editorState.position)), [editorState.position]);

  const total = Object.values(counts).reduce(
    (sum, side) => sum + Object.values(side).reduce((a, b) => a + b, 0), 0,
  );
  const invisibleEntries = (["black", "white"] as Color[]).flatMap((color) =>
    kinds
      .filter(([kind]) => counts[color][kind] > 0)
      .map(([kind, label]) => ({ color, kind, label, count: counts[color][kind] })),
  );
  const problemJson = buildProblemJson(encodeSfen(editorState.position), plies, rule, counts);
  const clear = () => { setError(undefined); setResponse(undefined); setSolvedProblem(undefined); };

  const changeCount = (color: Color, kind: InvisibleKind, delta: number) => {
    if (delta > 0 && total >= 2) return;
    setCounts((current) => ({
      ...current,
      [color]: { ...current[color], [kind]: Math.max(0, current[color][kind] + delta) },
    }));
    clear();
  };
  const edit = (event: Parameters<typeof dispatch>[0]) => { dispatch(event); clear(); };
  const selectMode = (mode: InputMode) => {
    if (mode === "json" && inputMode === "form") setManualProblem(problemJson);
    setInputMode(mode);
    clear();
  };
  const loadSfen = () => {
    try {
      dispatch({ ty: "set-position", position: decodeSfen(sfenInput) });
      clear();
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
  };
  const loadPreset = (sfen: string) => {
    dispatch({ ty: "set-position", position: decodeSfen(sfen) });
    setCounts(emptyCounts());
    clear();
  };
  const loadJsonIntoForm = () => {
    try {
      const problem = JSON.parse(manualProblem) as ProblemDocument;
      if (!problem || typeof problem.baseSfen !== "string" || !Array.isArray(problem.invisibles))
        throw new Error("問題JSONにbaseSfenとinvisiblesが必要です");
      if (!Number.isInteger(problem.plies) || problem.plies < 1)
        throw new Error("pliesは1以上の整数で指定してください");
      if (problem.rule && problem.rule !== "helpmate" && problem.rule !== "helpSelfmate")
        throw new Error("未知のルールです");
      const nextCounts = emptyCounts();
      let nextTotal = 0;
      for (const spec of problem.invisibles) {
        if ((spec.color !== "black" && spec.color !== "white") ||
            !kinds.some(([kind]) => kind === spec.kind) ||
            !Number.isInteger(spec.count) || spec.count < 0)
          throw new Error("invisiblesの所属・駒種・枚数を確認してください");
        nextCounts[spec.color][spec.kind] += spec.count;
        nextTotal += spec.count;
      }
      if (nextTotal > 2) throw new Error("透明駒（駒種指定）は合計2枚まで指定できます");
      dispatch({ ty: "set-position", position: decodeSfen(problem.baseSfen) });
      setCounts(nextCounts);
      setPlies(problem.plies);
      setRule(problem.rule ?? "helpmate");
      setInputMode("form");
      clear();
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
  };
  const solve = async () => {
    if (solving) { client.current?.cancel(); setSolving(false); return; }
    clear();
    setSolving(true);
    try {
      client.current ??= new KnownInvisibleSolverClient();
      const requestedProblem = inputMode === "form" ? problemJson : manualProblem;
      const json = await client.current.solve(requestedProblem, maxSolutions);
      if (json) {
        setResponse(JSON.parse(json) as SolveResponse);
        setSolvedProblem(JSON.parse(requestedProblem) as ProblemDocument);
      }
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setSolving(false); }
  };
  const shift = (direction: ShiftDirection) => edit({ ty: "shift", dir: direction });

  const solveControls = <VariableSolveControls
    idPrefix="known-invisible"
    plies={inputMode === "form" ? plies : undefined}
    setPlies={inputMode === "form" ? (value) => { setPlies(value); clear(); } : undefined}
    rule={inputMode === "form" ? rule : undefined}
    setRule={inputMode === "form" ? (value) => { setRule(value); clear(); } : undefined}
    ruleOptions={["helpmate", "helpSelfmate"]}
    maxSolutions={maxSolutions}
    setMaxSolutions={setMaxSolutions}
    solving={solving}
    onSolve={solve}
  />;
  const results = <>
    {error && <Alert variant="danger">{error}</Alert>}
    {response && solvedProblem && <>
      <p className="mb-0">{solvedProblem.rule === "helpSelfmate" ? "協力自玉詰" : "協力詰"} {solvedProblem.plies}手</p>
      <p>{formatInvisibleSummary(solvedProblem)}</p>
      <p>初形候補世界: <strong>{response.worldCount}</strong> ／ 解数: <strong>{response.solutions.length}</strong></p>
      {response.solutions.length === 0 ? <Alert variant="info">指定手数以下の解はありません。</Alert> :
        <ol className="known-invisible-solutions">{response.solutions.map((solution, index) => <li key={index}><code>{solution.join(" ")} まで {solution.length}手</code></li>)}</ol>}</>}
  </>;

  return <Card className="mb-4 known-invisible-solver">
    <Card.Header
      as="button"
      type="button"
      className="solver-card-toggle d-flex align-items-center justify-content-between text-start"
      aria-label={open ? "透明駒（駒種指定）の入力を閉じる" : "透明駒（駒種指定）の入力を開く"}
      aria-expanded={open}
      onClick={() => setOpen((current) => !current)}
    >
      <h2 className="h5 mb-0">透明駒（駒種指定）</h2>
      <span className="text-secondary">
        {open ? <BsChevronUp aria-hidden="true" /> : <BsChevronDown aria-hidden="true" />}
      </span>
    </Card.Header>
    {open && <Card.Body>
      <p className="small text-muted">駒種と所属が分かっていて、位置だけが不明な透明駒です。合計2枚まで指定できます。</p>
      <div className="d-flex flex-wrap align-items-center gap-2 mb-3">
        <ButtonGroup aria-label="透明駒（駒種指定）の問題入力方法">
          <Button variant={inputMode === "form" ? "primary" : "outline-primary"} onClick={() => selectMode("form")}>盤面・フォーム</Button>
          <Button variant={inputMode === "json" ? "primary" : "outline-primary"} onClick={() => selectMode("json")}>JSON詳細編集</Button>
        </ButtonGroup>
      </div>
      {inputMode === "form" ? <Row className="g-4 variable-three-column-layout">
        <Col xl={4} className="variable-layout-board">
          <VariablePositionEditor position={editorState.position} normalSelected={editorState.selected} variables={[]}
            clickBoard={(position) => edit({ ty: "click-board", pos: position })}
            rightClickBoard={(position) => edit({ ty: "right-click-board", pos: position })}
            clickKnownHand={(color, kind) => edit({ ty: "click-hand", color, kind: kind || undefined })}
            moveSelectedToHand={() => undefined} selectVariable={() => undefined} shiftBoard={shift} />
        </Col>
        <Col xl={4} className="variable-layout-settings">
          <Form.Group className="mb-3" controlId="known-invisible-base-sfen"><Form.Label>通常駒のbase SFEN</Form.Label>
            <div className="d-flex gap-2"><Form.Control className="variable-sfen-input" size="sm" value={sfenInput} onChange={(event) => setSfenInput(event.target.value)} spellCheck={false} />
              <Button className="text-nowrap" size="sm" variant="outline-secondary" onClick={loadSfen}>読込</Button></div>
          </Form.Group>
          <div className="d-flex flex-wrap gap-2 mb-3" aria-label="透明駒（駒種指定）の初形プリセット">
            <Button size="sm" variant="outline-secondary" onClick={() => loadPreset("4k4/9/9/9/9/9/9/9/9 b 2r2b4g4s4n4l18p 1")}>単玉のみ</Button>
            <Button size="sm" variant="outline-secondary" onClick={() => loadPreset(initialSfen)}>双玉のみ</Button>
            <Button size="sm" variant="outline-secondary" onClick={() => loadPreset("9/9/9/9/9/9/9/9/9 b - 1")}>すべて駒箱</Button>
          </div>
          <div className="known-invisible-control-panel border rounded p-2">
            <div className="d-flex flex-wrap align-items-end gap-2">
              <Form.Group className="known-invisible-owner" controlId="known-invisible-owner">
                <Form.Label>所属</Form.Label>
                <Form.Select size="sm" value={draftColor} disabled={solving} onChange={(event) => setDraftColor(event.target.value as Color)}>
                  <option value="black">攻方</option>
                  <option value="white">受方</option>
                </Form.Select>
              </Form.Group>
              <Form.Group className="known-invisible-kind" controlId="known-invisible-kind">
                <Form.Label>駒種</Form.Label>
                <Form.Select size="sm" value={draftKind} disabled={solving} onChange={(event) => setDraftKind(event.target.value as InvisibleKind)}>
                  {kinds.map(([kind, label]) => <option value={kind} key={kind}>{label}</option>)}
                </Form.Select>
              </Form.Group>
              <Button size="sm" className="text-nowrap" aria-label="透明駒を追加" disabled={total >= 2 || solving} onClick={() => changeCount(draftColor, draftKind, 1)}>追加</Button>
            </div>
            <div className="mt-2">
              <div className="small text-muted mb-1">追加済み（{total}/2枚）</div>
              {invisibleEntries.length === 0 ? <div className="small text-muted">なし</div> :
                <div className="known-invisible-added-list">{invisibleEntries.map(({ color, kind, label, count }) =>
                  <span className="known-invisible-added-item" key={`${color}-${kind}`}>
                    <span>{color === "black" ? "攻方" : "受方"} {label} ×{count}</span>
                    <button type="button" className="btn-close" aria-label={`${color === "black" ? "攻方" : "受方"}${label}を1枚削除`} disabled={solving} onClick={() => changeCount(color, kind, -1)} />
                  </span>,
                )}</div>}
            </div>
          </div>
        </Col>
        <Col xl={4} className="variable-layout-results">{solveControls}{results}</Col>
      </Row> : <div>
        <Form.Group className="mb-2" controlId="known-invisible-problem-json"><Form.Label>問題JSON</Form.Label>
          <Form.Control as="textarea" className="variable-problem-json" value={manualProblem} onChange={(event) => { setManualProblem(event.target.value); clear(); }} spellCheck={false} />
        </Form.Group>
        <Button variant="outline-secondary" onClick={loadJsonIntoForm}>JSONをフォームに反映</Button>
        <div className="my-3">{solveControls}{results}</div>
      </div>}
    </Card.Body>}
  </Card>;
}
