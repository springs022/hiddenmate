export interface VariableSolveRequest {
  type: "solve";
  requestId: number;
  problemJson: string;
  maxSolutions: number;
}

export type VariableSolveWorkerResponse =
  | { type: "ready" }
  | {
      type: "solved";
      requestId: number;
      responseJson: string;
    }
  | {
      type: "error";
      requestId: number;
      message: string;
    };
