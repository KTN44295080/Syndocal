# KDMX development completion gate

- Changes that affect the native UI or runtime are not complete after a frontend-only build.
- Before handing off such changes, run `pnpm --dir app tauri build --no-bundle` successfully.
- Launch `target/release/syndocal.exe` after the build and verify that exactly one responsive `Syndocal` window is available.
- If an older release process is running, close that exact KDMX release process before replacing it. Prefer a graceful window close before forcing termination.
- Do not claim native verification when only browser harnesses, TypeScript, Vite, or Rust unit tests were run.

## UI sizing obligation

- Do not treat making UI elements smaller as a default improvement or routine density fix.
- Preserve existing typography, controls, icons, spacing, and hit-target sizes unless the user explicitly requests a smaller size for a named element.
- Solve space pressure with reflow, disclosure, pagination, or internal scrolling before considering any size reduction.
- Any explicitly requested size reduction must remain local to the named element; do not use it as permission to shrink adjacent or shared UI.
