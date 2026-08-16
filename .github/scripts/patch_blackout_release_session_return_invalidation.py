from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


security = Path("app/src-tauri/src/control_plane_security.rs")

# A challenge prepared while the workstation is locked or the session is remote
# cannot be allowed to survive the transition back to the ordinary local
# desktop. Revoke again on unlock / console reconnect; this also handles a
# prepare that occurred after the original lock/remote-connect event.
replace_exact(
    security,
    '''                WM_SYSKEYDOWN, WM_WTSSESSION_CHANGE, WTS_CONSOLE_DISCONNECT,
                WTS_REMOTE_CONNECT, WTS_REMOTE_DISCONNECT, WTS_SESSION_LOCK,
                WTS_SESSION_REMOTE_CONTROL,
''',
    '''                WM_SYSKEYDOWN, WM_WTSSESSION_CHANGE, WTS_CONSOLE_CONNECT,
                WTS_CONSOLE_DISCONNECT, WTS_REMOTE_CONNECT, WTS_REMOTE_DISCONNECT,
                WTS_SESSION_LOCK, WTS_SESSION_REMOTE_CONTROL, WTS_SESSION_UNLOCK,
''',
    "import session return invalidation constants",
)
replace_exact(
    security,
    '''                    WTS_SESSION_LOCK
                        | WTS_CONSOLE_DISCONNECT
                        | WTS_REMOTE_CONNECT
                        | WTS_REMOTE_DISCONNECT
                        | WTS_SESSION_REMOTE_CONTROL
''',
    '''                    WTS_SESSION_LOCK
                        | WTS_SESSION_UNLOCK
                        | WTS_CONSOLE_CONNECT
                        | WTS_CONSOLE_DISCONNECT
                        | WTS_REMOTE_CONNECT
                        | WTS_REMOTE_DISCONNECT
                        | WTS_SESSION_REMOTE_CONTROL
''',
    "invalidate consent on return to local desktop",
)
