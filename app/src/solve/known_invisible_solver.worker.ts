/// <reference lib="webworker" />

import { solve_known_invisible_problem } from "../wasm_api";
import {
  KnownInvisibleSolveRequest,
  KnownInvisibleSolveWorkerResponse,
} from "./known_invisible_solver_protocol";

const worker = self as DedicatedWorkerGlobalScope;
worker.onmessage = (event: MessageEvent<KnownInvisibleSolveRequest>) => {
  const request = event.data;
  let response: KnownInvisibleSolveWorkerResponse;
  try {
    response = {
      type: "solved",
      requestId: request.requestId,
      responseJson: solve_known_invisible_problem(
        request.problemJson,
        request.maxSolutions,
      ),
    };
  } catch (reason) {
    response = {
      type: "error",
      requestId: request.requestId,
      message: reason instanceof Error ? reason.message : String(reason),
    };
  }
  worker.postMessage(response);
};
worker.postMessage({ type: "ready" } satisfies KnownInvisibleSolveWorkerResponse);

export {};
