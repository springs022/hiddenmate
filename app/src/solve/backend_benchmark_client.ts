export type WasmBackendBenchmarkResult = {
  problemType: "variable" | "knownInvisible";
  backend: "explicit" | "replay";
  worldCount: number;
  solutionCount: number;
  initMillis: number;
  solveMillis: number;
  visitedStates: number;
  transitions: number;
  memoryBeforeBytes: number;
  memoryAfterInitBytes: number;
  memoryAfterSolveBytes: number;
};

type WorkerResponse =
  | { type: "ready" }
  | { type: "measured"; resultJson: string }
  | { type: "error"; message: string };

export function measureWasmBackend(
  problemJson: string,
  backend: "explicit" | "replay",
  maxSolutions: number,
): Promise<WasmBackendBenchmarkResult> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(
      new URL("./backend_benchmark.worker.ts", import.meta.url),
    );
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const response = event.data;
      if (response.type === "ready") {
        worker.postMessage({ problemJson, backend, maxSolutions });
      } else if (response.type === "measured") {
        worker.terminate();
        resolve(JSON.parse(response.resultJson) as WasmBackendBenchmarkResult);
      } else {
        worker.terminate();
        reject(new Error(response.message));
      }
    };
    worker.onerror = (event) => {
      worker.terminate();
      reject(new Error(event.message));
    };
  });
}
