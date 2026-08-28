import { runSelfTest } from "./check-asio-v3-contract.mjs";

const assertions = runSelfTest();
process.stdout.write("ASIO v3 contract tests passed: " + assertions + " assertions.\n");
