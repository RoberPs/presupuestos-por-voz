import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach } from "vitest";

// jsdom sustituye AbortSignal por el suyo y el Request nativo de Node lo rechaza
// ("Expected signal to be an instance of AbortSignal"), lo que rompe la navegación del
// data router de React Router en las pruebas. Se descarta la señal solo en este entorno.
const NativeRequest = globalThis.Request;
globalThis.Request = class extends NativeRequest {
  constructor(input: RequestInfo | URL, init?: RequestInit) {
    if (init?.signal) {
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { signal, ...rest } = init;
      super(input, rest);
    } else {
      super(input, init);
    }
  }
} as typeof Request;

afterEach(() => {
  cleanup();
  clearMocks();
});
