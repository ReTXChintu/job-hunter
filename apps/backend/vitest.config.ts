import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Tests use real bcrypt hashing at production cost and real sockets;
    // vitest's 5 s default is too tight on a loaded machine or CI runner.
    testTimeout: 20_000,
  },
});
