from __future__ import annotations

import subprocess
import shutil
import tempfile
from pathlib import Path


DOCUMENT_RECOVERY_PATTERNS = [
    "~/Library/Caches/Adobe InDesign/Version */*/InDesign Recovery",
    "~/Library/Caches/Adobe InDesign/Version */*/InDesign SavedData",
]
SCRIPTING_STATE_PATTERNS = [
    "~/Library/Caches/Adobe InDesign/Version */*/Scripting Support/*/Scripting SavedData",
]


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
    becomes scriptable. Killing the app first is more reliable than trying to
    click localized dialog buttons.
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


def run_indesign_case_command(
    command: list[str],
    cwd: Path,
    timeout: int,
) -> tuple[subprocess.CompletedProcess[str], bool]:
    try:
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
