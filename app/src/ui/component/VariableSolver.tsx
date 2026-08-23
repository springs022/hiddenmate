import { useState } from "react";
import { Alert, Button, Card, Form, Table } from "react-bootstrap";
import { solve_variable_problem } from "../../wasm_api";

const exampleProblem = `{
  "baseSfen": "9/9/kS7/N8/1L7/9/9/9/9 b - 1",
  "plies": 1,
  "variables": [
    {
      "id": 1,
      "color": "black",
      "square": "64",
      "candidates": ["R", "+R"]
    }
  ]
}`;

interface VariableCandidates {
  id: number;
  kinds: string[];
}

interface VariableSolveResponse {
  worldCount: number;
  candidates: VariableCandidates[];
  solutions: string[][];
}

export function VariableSolver() {
  const [problem, setProblem] = useState(exampleProblem);
  const [maxSolutions, setMaxSolutions] = useState(100);
  const [response, setResponse] = useState<VariableSolveResponse>();
  const [error, setError] = useState<string>();
  const [solving, setSolving] = useState(false);

  const solve = () => {
    setSolving(true);
    setError(undefined);
    setResponse(undefined);
    try {
      const json = solve_variable_problem(problem, maxSolutions);
      setResponse(JSON.parse(json) as VariableSolveResponse);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSolving(false);
    }
  };

  return (
    <Card className="mb-4">
      <Card.Header as="h2" className="h5 mb-0">
        覆面駒（Variable）検討
      </Card.Header>
      <Card.Body>
        <p>
          問題JSONを入力して、指定手数ちょうどの協力詰を検討します。最初は入力済みのサンプルをそのまま実行できます。
        </p>
        <Form.Group className="mb-3" controlId="variable-problem-json">
          <Form.Label>問題JSON</Form.Label>
          <Form.Control
            as="textarea"
            className="variable-problem-json"
            value={problem}
            onChange={(event) => setProblem(event.target.value)}
            disabled={solving}
            spellCheck={false}
          />
        </Form.Group>
        <div className="d-flex align-items-end gap-3 mb-3">
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
        {response && (
          <div aria-live="polite">
            <p>
              初形候補世界: <strong>{response.worldCount}</strong> ／ 解数: <strong>{response.solutions.length}</strong>
            </p>
            <Table bordered size="sm" className="variable-result-table">
              <thead>
                <tr>
                  <th>覆面駒</th>
                  <th>残る候補</th>
                </tr>
              </thead>
              <tbody>
                {response.candidates.map((candidate) => (
                  <tr key={candidate.id}>
                    <td>V{candidate.id}</td>
                    <td>{candidate.kinds.join(", ") || "なし"}</td>
                  </tr>
                ))}
              </tbody>
            </Table>
            <h3 className="h6">解答</h3>
            {response.solutions.length === 0 ? (
              <p>解なし</p>
            ) : (
              <ol>
                {response.solutions.map((solution, index) => (
                  <li key={`${index}-${solution.join("-")}`}>
                    <code>{solution.join(" ")}</code>
                  </li>
                ))}
              </ol>
            )}
          </div>
        )}
      </Card.Body>
    </Card>
  );
}
