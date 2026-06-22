from __future__ import annotations

from contextlib import contextmanager
import subprocess
import shutil
import tempfile
import threading
from pathlib import Path


DOCUMENT_RECOVERY_PATTERNS = [
    "~/Library/Caches/Adobe InDesign/Version */*/InDesign Recovery",
    "~/Library/Caches/Adobe InDesign/Version */*/InDesign SavedData",
    "~/Library/Preferences/Adobe InDesign/Version */*/InDesign Recovery",
    "~/Library/Preferences/Adobe InDesign/Version */*/InDesign SavedData",
    "~/Library/Application Support/Adobe/Adobe InDesign/Version */*/InDesign Recovery",
    "~/Library/Application Support/Adobe/Adobe InDesign/Version */*/InDesign SavedData",
    "~/Library/Saved Application State/com.adobe.InDesign.savedState",
]
SCRIPTING_STATE_PATTERNS = [
    "~/Library/Caches/Adobe InDesign/Version */*/Scripting Support/*/Scripting SavedData",
]

INDESIGN_PROCESS_NAMES = [
    "Adobe InDesign 2026",
    "Adobe InDesign 2025",
    "Adobe InDesign",
]

RECOVERY_DISMISS_BUTTONS = [
    "Don't Recover",
    "Don’t Recover",
    "Do Not Recover",
    "Do not Recover",
    "No",
    "Nein",
    "Nicht wiederherstellen",
    "Nicht Wiederherstellen",
    "Abbrechen",
    "Cancel",
    "Discard",
    "Verwerfen",
]

RECOVERY_MODAL_WATCH_INTERVAL = 1.0


def kill_indesign() -> None:
    subprocess.run(
        ["pkill", "-9", "-x", "Adobe InDesign 2026"],
        text=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    subprocess.run(
        ["pkill", "-9", "-f", "Adobe InDesign 2026.app"],
        text=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def prepare_indesign_for_automation() -> list[Path]:
    """Start automation from a clean app state.

    After a crash, InDesign can reopen into a recovery modal before ExtendScript
    becomes scriptable. Killing the app first removes the live process; clearing
    persisted state prevents most restore prompts on the next launch.
    """
    kill_indesign()
    return clear_indesign_recovery_state(include_scripting_state=True)


def clear_indesign_recovery_state(include_scripting_state: bool = False) -> list[Path]:
    removed = []
    patterns = list(DOCUMENT_RECOVERY_PATTERNS)
    if include_scripting_state:
        patterns.extend(SCRIPTING_STATE_PATTERNS)
    for pattern in patterns:
        for path in Path.home().glob(pattern.removeprefix("~/")):
            if not path.exists():
                continue
            try:
                if path.is_dir():
                    shutil.rmtree(path)
                else:
                    path.unlink()
                removed.append(path)
            except OSError:
                continue
    return removed


def reset_indesign_after_failure() -> list[Path]:
    kill_indesign()
    return clear_indesign_recovery_state(include_scripting_state=True)


def cleanup_indesign_automation_state() -> list[Path]:
    """Remove any modal-prone InDesign state left after an automation run."""
    kill_indesign()
    return clear_indesign_recovery_state(include_scripting_state=True)


def run_indesign_case_command(
    command: list[str],
    cwd: Path,
    timeout: int,
) -> tuple[subprocess.CompletedProcess[str], bool]:
    try:
        with indesign_recovery_modal_watch():
            result = subprocess.run(
                command,
                cwd=cwd,
                text=True,
                capture_output=True,
                timeout=timeout,
            )
        return result, False
    except subprocess.TimeoutExpired as error:
        stderr = _timeout_text(error.stderr)
        if stderr and not stderr.endswith("\n"):
            stderr += "\n"
        stderr += f"InDesign automation timed out after {timeout}s."
        return (
            subprocess.CompletedProcess(
                command,
                124,
                stdout=_timeout_text(error.stdout),
                stderr=stderr,
            ),
            True,
        )


def _timeout_text(value: str | bytes | None) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value


@contextmanager
def indesign_recovery_modal_watch():
    """Best-effort UI cleanup while InDesign starts.

    InDesign's crash recovery prompt can appear after the process has launched
    and before ExtendScript is accepted. File cleanup handles the common case;
    this watcher handles the already-visible localized modal without changing
    the rendering scripts.
    """
    stop = threading.Event()
    thread = threading.Thread(
        target=_watch_indesign_recovery_modal,
        args=(stop,),
        name="indesign-recovery-modal-watch",
        daemon=True,
    )
    thread.start()
    dismiss_indesign_recovery_modal()
    try:
        yield
    finally:
        stop.set()
        thread.join(timeout=2)


def _watch_indesign_recovery_modal(stop: threading.Event) -> None:
    while not stop.wait(RECOVERY_MODAL_WATCH_INTERVAL):
        dismiss_indesign_recovery_modal()


def dismiss_indesign_recovery_modal() -> bool:
    """Click known negative buttons in InDesign's crash-recovery dialog.

    The exact button label is localized. We intentionally avoid generic "OK" or
    positive restore buttons, because these runs may discard any user documents
    but should never restore stale benchmark documents into the automation path.
    """
    script = _dismiss_modal_applescript()
    try:
        result = subprocess.run(
            ["osascript", "-e", script],
            text=True,
            capture_output=True,
            timeout=3,
        )
    except (OSError, subprocess.SubprocessError):
        return False
    return result.returncode == 0 and result.stdout.strip() == "1"


def _dismiss_modal_applescript() -> str:
    process_names = ", ".join(_applescript_string(name) for name in INDESIGN_PROCESS_NAMES)
    button_names = ", ".join(_applescript_string(name) for name in RECOVERY_DISMISS_BUTTONS)
    return f"""
set processNames to {{{process_names}}}
set dismissButtonNames to {{{button_names}}}
set didClick to 0
tell application "System Events"
    repeat with processName in processNames
        if exists process processName then
            tell process processName
                repeat with dialogWindow in windows
                    repeat with buttonName in dismissButtonNames
                        try
                            if exists button buttonName of dialogWindow then
                                click button buttonName of dialogWindow
                                set didClick to 1
                                return didClick
                            end if
                        end try
                    end repeat
                end repeat
            end tell
        end if
    end repeat
end tell
return didClick
"""


def _applescript_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def run_indesign_preflight(root: Path, timeout: int, baseline_kind: str) -> None:
    prepare_indesign_for_automation()
    with tempfile.NamedTemporaryFile("w", suffix=".jsx", delete=False, encoding="utf-8") as handle:
        handle.write(
            "#target indesign\n"
            "app.scriptPreferences.userInteractionLevel = UserInteractionLevels.NEVER_INTERACT;\n"
            "app.documents.length;\n"
        )
        script_path = Path(handle.name)
    try:
        with indesign_recovery_modal_watch():
            result = subprocess.run(
                ["osascript", str(root / "scripts/run-indesign-export.scpt"), str(script_path)],
                cwd=root,
                text=True,
                capture_output=True,
                timeout=timeout,
            )
    except subprocess.TimeoutExpired as error:
        reset_indesign_after_failure()
        raise SystemExit(
            "InDesign automation preflight timed out. "
            f"InDesign was killed so the suite does not write {baseline_kind} render-error baselines.\n"
            f"stdout:\n{error.stdout or ''}\nstderr:\n{error.stderr or ''}"
        ) from error
    finally:
        script_path.unlink(missing_ok=True)

    if result.returncode != 0:
        reset_indesign_after_failure()
        raise SystemExit(
            "InDesign automation preflight failed. "
            f"No {baseline_kind} baseline was written because InDesign is not scriptable right now.\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
