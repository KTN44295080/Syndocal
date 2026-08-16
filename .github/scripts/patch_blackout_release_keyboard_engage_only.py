from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


shortcuts = Path("app/src/appShortcutActions.ts")

# The B key is a safety-direction shortcut only. Once blackout is engaged it
# must not synthesize an energizing clear intent; R4 Release remains the only
# local clear path and requires fresh physical consent.
replace_exact(
    shortcuts,
    '  if (event.code === "KeyB") return { kind: "toggleBlackout", enabled: !context.blackout };\n',
    '  if (event.code === "KeyB") return { kind: "toggleBlackout", enabled: true };\n',
    "Control B shortcut is blackout-engage-only",
)
