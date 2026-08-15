from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


app = Path("app/src/App.tsx")
replace_exact(
    app,
    '''  const setAllBlackout = async (enabled: boolean) => {
    try {
      if (!enabled) {
        setMessage("All blackout clear requires physical DMX Blackout Release in Runtime controls.");
        return;
      }
      await invoke("set_all_blackout", { enabled: true });
      setMessage("All blackout enabled.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };
''',
    '''  const setAllBlackout = async (enabled: boolean) => {
    try {
      if (!enabled) {
        setMessage("All blackout clear requires physical DMX Blackout Release in Runtime controls.");
        return;
      }
      // Combined blackout must use the same sticky S0 DMX latch as the
      // dedicated DMX button. Otherwise the R4 Release control could succeed
      // as a no-op while a separate legacy blackout remained asserted.
      await safetyBlackoutRuntime.engage();
      await setVideoBlackout(true);
      await refreshSnapshot();
      if (!snapshot().blackout || !snapshot().video.blackout) {
        throw new Error("All blackout did not reach the combined DMX/video safe state.");
      }
      setMessage("All blackout enabled.");
    } catch (error) {
      setMessage(String(error));
    }
  };
''',
    "All Blackout S0 convergence",
)
