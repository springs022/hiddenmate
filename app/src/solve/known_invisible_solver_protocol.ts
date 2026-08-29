export interface KnownInvisibleSolveRequest {
  type: "solve";
  requestId: number;
  problemJson: string;
  maxSolutions: number;
}

export type KnownInvisibleSolveWorkerResponse =
  | { type: "ready" }
  | { type: "solved"; requestId: number; responseJson: string }
  | { type: "error"; requestId: number; message: string };
