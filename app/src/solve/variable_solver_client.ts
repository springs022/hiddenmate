import {
  VariableSolveRequest,
  VariableSolveWorkerResponse,
} from "./variable_solver_protocol";

interface PendingSolve {
  requestId: number;
  request: VariableSolveRequest;
  resolve: (responseJson: string | undefined) => void;
  reject: (reason: Error) => void;
}

export class VariableSolverClient {
  private worker: Worker | undefined;
  private workerReady = false;
  private pending: PendingSolve | undefined;
  private nextRequestId = 1;

  solve(
    problemJson: string,
    maxSolutions: number,
    hideRedundantDefenses: boolean,
  ): Promise<string | undefined> {
    if (this.pending) {
      throw new Error("検討は既に実行中です。");
    }

    const worker = this.ensureWorker();
    const requestId = this.nextRequestId++;
    const request: VariableSolveRequest = {
      type: "solve",
      requestId,
      problemJson,
      maxSolutions,
      hideRedundantDefenses,
    };

    return new Promise((resolve, reject) => {
      this.pending = { requestId, request, resolve, reject };
      if (this.workerReady) {
        this.postPending(worker);
      }
    });
  }

  cancel(): void {
    const pending = this.pending;
    this.pending = undefined;
    this.disposeWorker();
    pending?.resolve(undefined);
  }

  dispose(): void {
    this.cancel();
  }

  private ensureWorker(): Worker {
    if (this.worker) {
      return this.worker;
    }

    const worker = new Worker(
      new URL("./variable_solver.worker.ts", import.meta.url),
    );
    this.workerReady = false;
    worker.onmessage = (event: MessageEvent<VariableSolveWorkerResponse>) => {
      const response = event.data;
      if (response.type === "ready") {
        this.workerReady = true;
        this.postPending(worker);
        return;
      }

      const pending = this.pending;
      if (!pending || response.requestId !== pending.requestId) {
        return;
      }

      this.pending = undefined;
      if (response.type === "solved") {
        pending.resolve(response.responseJson);
      } else {
        pending.reject(new Error(response.message));
      }
    };
    worker.onerror = (event) => {
      this.failWorker(
        new Error(event.message || "検討用Workerでエラーが発生しました。"),
      );
    };
    worker.onmessageerror = () => {
      this.failWorker(new Error("検討結果を読み取れませんでした。"));
    };
    this.worker = worker;
    return worker;
  }

  private failWorker(reason: Error): void {
    const pending = this.pending;
    this.pending = undefined;
    this.disposeWorker();
    pending?.reject(reason);
  }

  private postPending(worker: Worker): void {
    const pending = this.pending;
    if (!pending) {
      return;
    }

    try {
      worker.postMessage(pending.request);
    } catch (reason) {
      this.failWorker(
        reason instanceof Error
          ? reason
          : new Error("検討を開始できませんでした。"),
      );
    }
  }

  private disposeWorker(): void {
    this.worker?.terminate();
    this.worker = undefined;
    this.workerReady = false;
  }
}
