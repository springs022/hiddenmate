import { benchmark_candidate_backend } from "../wasm_api";

type BenchmarkRequest = {
  problemJson: string;
  backend: "explicit" | "replay";
  maxSolutions: number;
};

type BenchmarkResponse =
  | { type: "ready" }
  | { type: "measured"; resultJson: string }
  | { type: "error"; message: string };

const worker = self;
worker.onmessage = (event: MessageEvent<BenchmarkRequest>) => {
  try {
    worker.postMessage({
      type: "measured",
      resultJson: benchmark_candidate_backend(
        event.data.problemJson,
        event.data.backend,
        event.data.maxSolutions,
      ),
    } satisfies BenchmarkResponse);
  } catch (reason) {
    worker.postMessage({
      type: "error",
      message:
        reason instanceof Error ? (reason.stack ?? reason.message) : String(reason),
    } satisfies BenchmarkResponse);
  }
};
worker.postMessage({ type: "ready" } satisfies BenchmarkResponse);

export {};
