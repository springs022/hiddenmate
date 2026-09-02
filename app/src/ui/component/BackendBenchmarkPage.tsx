import { useEffect, useRef, useState } from "react";
import {
  measureWasmBackend,
  WasmBackendBenchmarkResult,
} from "../../solve/backend_benchmark_client";

type Scenario = {
  name: string;
  problem: object;
  maxSolutions: number;
};

type Summary = {
  scenario: string;
  backend: "explicit" | "replay";
  runs: number;
  worldCount: number;
  solutionCount: number;
  medianInitMillis: number;
  medianSolveMillis: number;
  medianInitialMemoryMiB: number;
  medianFinalMemoryMiB: number;
};

const scenarios: Scenario[] = [
  {
    name: "variable2-1ply",
    maxSolutions: 100,
    problem: {
      baseSfen: "9/9/kS7/9/1L7/9/9/9/9 b 2r2b4g3s4n3l18p 1",
      plies: 1,
      rule: "helpmate",
      handVariableMode: "indistinguishable",
      variables: [1, 2].map((id) => ({
        id,
        color: "black",
        inHand: true,
        candidates: ["P", "L", "N", "S", "G", "B", "R"],
      })),
    },
  },
  {
    name: "variable6-init",
    maxSolutions: 0,
    problem: {
      baseSfen: "8k/9/9/9/9/9/9/9/9 b - 1",
      plies: 1,
      rule: "helpmate",
      variables: ["99", "89", "79", "69", "59", "49"].map(
        (square, index) => ({
          id: index + 1,
          color: "black",
          square,
          candidates: ["P", "L", "N", "S"],
        }),
      ),
    },
  },
  {
    name: "invisible1-5ply",
    maxSolutions: 2,
    problem: {
      baseSfen: "7k1/9/7K1/9/9/9/9/9/9 b - 1",
      plies: 5,
      rule: "helpmate",
      invisibles: [{ color: "black", kind: "L", count: 1 }],
    },
  },
  {
    name: "invisible2-1ply",
    maxSolutions: 1,
    problem: {
      baseSfen: "7k1/9/7K1/9/9/9/9/9/9 b - 1",
      plies: 1,
      rule: "helpmate",
      invisibles: [
        { color: "black", kind: "L", count: 1 },
        { color: "white", kind: "R", count: 1 },
      ],
    },
  },
];

const median = (values: number[]): number => {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
};

export default function BackendBenchmarkPage() {
  const started = useRef(false);
  const [status, setStatus] = useState("準備中");
  const [summaries, setSummaries] = useState<Summary[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    const runs = Number(new URLSearchParams(location.search).get("runs") ?? 5);
    void (async () => {
      const result: Summary[] = [];
      for (const scenario of scenarios) {
        for (const backend of ["explicit", "replay"] as const) {
          const measurements: WasmBackendBenchmarkResult[] = [];
          for (let iteration = 1; iteration <= runs; iteration += 1) {
            setStatus(`${scenario.name} ${backend} ${iteration}/${runs}`);
            measurements.push(
              await measureWasmBackend(
                JSON.stringify(scenario.problem),
                backend,
                scenario.maxSolutions,
              ),
            );
          }
          result.push({
            scenario: scenario.name,
            backend,
            runs,
            worldCount: measurements[0].worldCount,
            solutionCount: measurements[0].solutionCount,
            medianInitMillis: median(measurements.map((item) => item.initMillis)),
            medianSolveMillis: median(
              measurements.map((item) => item.solveMillis),
            ),
            medianInitialMemoryMiB:
              median(measurements.map((item) => item.memoryAfterInitBytes)) /
              1024 /
              1024,
            medianFinalMemoryMiB:
              median(measurements.map((item) => item.memoryAfterSolveBytes)) /
              1024 /
              1024,
          });
        }
      }
      setSummaries(result);
      setStatus("完了");
    })().catch((reason) => {
      setError(reason instanceof Error ? reason.message : String(reason));
      setStatus("失敗");
    });
  }, []);

  return (
    <main style={{ padding: 24, fontFamily: "monospace" }}>
      <h1>Wasm候補世界バックエンド ベンチマーク</h1>
      <p id="backend-benchmark-status">{status}</p>
      {error && <pre id="backend-benchmark-error">{error}</pre>}
      {summaries && (
        <pre id="backend-benchmark-results">
          {JSON.stringify(summaries, null, 2)}
        </pre>
      )}
    </main>
  );
}
