/// <reference lib="webworker" />

import { solve_variable_problem } from "../wasm_api";
import {
  VariableSolveRequest,
  VariableSolveWorkerResponse,
} from "./variable_solver_protocol";

const worker = self as DedicatedWorkerGlobalScope;

worker.onmessage = (event: MessageEvent<VariableSolveRequest>) => {
  const request = event.data;
  if (request.type !== "solve") {
    return;
  }

  let response: VariableSolveWorkerResponse;
  try {
    response = {
      type: "solved",
      requestId: request.requestId,
      responseJson: solve_variable_problem(
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
const ready: VariableSolveWorkerResponse = { type: "ready" };
worker.postMessage(ready);

export {};
