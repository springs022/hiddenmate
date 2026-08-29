import {
  KnownInvisibleSolveRequest,
  KnownInvisibleSolveWorkerResponse,
} from "./known_invisible_solver_protocol";

export class KnownInvisibleSolverClient {
  private worker?: Worker;
  private ready = false;
  private pending?: {
    request: KnownInvisibleSolveRequest;
    resolve: (value: string | undefined) => void;
    reject: (reason: Error) => void;
  };
  private nextRequestId = 1;

  solve(problemJson: string, maxSolutions: number): Promise<string | undefined> {
    if (this.pending) throw new Error("検討は既に実行中です。");
    const worker = this.ensureWorker();
    const request: KnownInvisibleSolveRequest = {
      type: "solve",
      requestId: this.nextRequestId++,
      problemJson,
      maxSolutions,
    };
    return new Promise((resolve, reject) => {
      this.pending = { request, resolve, reject };
      if (this.ready) worker.postMessage(request);
    });
  }

  cancel(): void {
    const pending = this.pending;
    this.pending = undefined;
    this.worker?.terminate();
    this.worker = undefined;
    this.ready = false;
    pending?.resolve(undefined);
  }

  dispose(): void {
    this.cancel();
  }

  private ensureWorker(): Worker {
    if (this.worker) return this.worker;
    const worker = new Worker(
      new URL("./known_invisible_solver.worker.ts", import.meta.url),
    );
    worker.onmessage = (
      event: MessageEvent<KnownInvisibleSolveWorkerResponse>,
    ) => {
      const response = event.data;
      if (response.type === "ready") {
        this.ready = true;
        if (this.pending) worker.postMessage(this.pending.request);
        return;
      }
      const pending = this.pending;
      if (!pending || pending.request.requestId !== response.requestId) return;
      this.pending = undefined;
      if (response.type === "solved") pending.resolve(response.responseJson);
      else pending.reject(new Error(response.message));
    };
    worker.onerror = (event) => {
      const pending = this.pending;
      this.pending = undefined;
      this.cancel();
      pending?.reject(new Error(event.message || "検討用Workerでエラーが発生しました。"));
    };
    this.worker = worker;
    return worker;
  }
}
