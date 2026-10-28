// Local certificate/PFX signing is retired. Final release acceptance uses the
// trusted SignPath workflow or an explicit unsigned declaration and final bytes.
import { runWindowsSigningStateCli } from "./sign-windows-release-state.mjs";

if (process.argv.slice(2).some((flag) => ["--pfx", "--thumbprint", "--timestamp", "--artifact"].includes(flag))) {
  throw new Error("Local Authenticode signing is retired. Use sign-windows-release-state.mjs with --installer, --payload-root and an explicit SignPath or unsigned state; SignPath requests must originate in the trusted GitHub build.");
}
runWindowsSigningStateCli(process.argv.slice(2));
