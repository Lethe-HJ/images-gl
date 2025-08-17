// Worker 类型声明

declare module "*.ts?worker" {
  const workerConstructor: {
    new (): Worker;
  };
  export default workerConstructor;
}

declare module "*.ts?sharedworker" {
  const sharedWorkerConstructor: {
    new (): SharedWorker;
  };
  export default sharedWorkerConstructor;
}

// 扩展 Worker 类型以支持 import.meta.url
declare global {
  interface WorkerOptions {
    type?: "classic" | "module";
  }
}

export {};
