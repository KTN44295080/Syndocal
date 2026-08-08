# KDMX development completion gate

- Changes that affect the native UI or runtime are not complete after a frontend-only build.
- Immediately before every native release build, find any running process whose resolved executable path is exactly this checkout's `target/release/syndocal.exe`, verify that exact path, and force-terminate only that process. Do this proactively so the linker can replace the executable; do not wait for an access-denied build failure. Never terminate Daslight or an unrelated `syndocal.exe` from another checkout.
- Before handing off such changes, run `pnpm --dir app tauri build --no-bundle` successfully.
- Launch `target/release/syndocal.exe` after the build and verify that exactly one responsive `Syndocal` window is available.
- Do not claim native verification when only browser harnesses, TypeScript, Vite, or Rust unit tests were run.

## UI sizing obligation

- Do not treat making UI elements smaller as a default improvement or routine density fix.
- Preserve existing typography, controls, icons, spacing, and hit-target sizes unless the user explicitly requests a smaller size for a named element.
- Solve space pressure with reflow, disclosure, pagination, or internal scrolling before considering any size reduction.
- Any explicitly requested size reduction must remain local to the named element; do not use it as permission to shrink adjacent or shared UI.
