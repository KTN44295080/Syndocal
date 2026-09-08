# Native thumbnail cancellation revalidation — 2026-09-09

This is a current-`main` revalidation checkpoint for the existing native
thumbnail cancellation implementation. It does not add product behavior or
rerun the prohibited missing-file move helper.

## Source and environment

- HEAD: `165e355399a4cc1efde9cd3624cf61d93a4a98a7`
- Product source changes: none in this checkpoint
- Cargo environment: `vcvars64.bat -vcvars_ver=14.44`
- Cargo linker pin:
  `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
- `where.exe link.exe`: the pinned linker was first

## Results

The native focused command exited 0:

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --locked -j 1 native_thumbnail -- --test-threads=1
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 1791 filtered out
```

The 23 tests cover real PNG/MP4 thumbnail paths, normal and cancellable
rendering, exact ticket ownership and lane targeting, independent lanes,
Channel serialization and announcement failure, cancellation while running or
queued, slot retention until worker return, late-success rejection, renderer
wait expiry/cancellation, poisoned-lock rejection, and worker unwind/admission
cleanup. The captured command log is kept at
`target/qa/native-thumbnail-cancel-20260909-01/native-thumbnail-tests.log`.

The frontend contracts also passed on this HEAD:

- `pnpm.cmd --dir app run check:native-thumbnail-request`
- `pnpm.cmd --dir app run check:media-thumbnails`

These checks prove the existing software/native contract only. They do not
prove a user-facing missing-file → Retry button → recovery run, a hard stop
deadline for synchronous decoder/OS I/O, physical output, device acceptance,
Mac execution, signing, publication, or product-wide completion.
