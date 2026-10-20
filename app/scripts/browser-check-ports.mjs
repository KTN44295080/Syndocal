import assert from "node:assert/strict";
import { createServer } from "node:net";

// Keep ephemeral ports by default; parallel worktrees can pin their assigned
// Vite/CDP ports. Binding first fails closed if another process owns the port.
export async function reserveBrowserCheckPort(environmentVariable, defaultPort = 0) {
  const raw = process.env[environmentVariable] ?? String(defaultPort);
  const requestedPort = Number(raw);
  assert.ok(
    /^(0|[1-9]\d*)$/.test(raw)
      && Number.isSafeInteger(requestedPort)
      && (requestedPort === 0 || (requestedPort >= 1024 && requestedPort <= 65535)),
    `Invalid ${environmentVariable}: expected 0 or a port from 1024 to 65535`,
  );
  const reservation = createServer();
  await new Promise((done, reject) => {
    reservation.once("error", reject);
    reservation.listen(requestedPort, "127.0.0.1", done);
  });
  const port = reservation.address().port;
  await new Promise((done, reject) => {
    reservation.close((error) => error ? reject(error) : done());
  });
  return port;
}
