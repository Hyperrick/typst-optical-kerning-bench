from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path


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


def run_indesign_preflight(root: Path, timeout: int, baseline_kind: str) -> None:
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
        kill_indesign()
        raise SystemExit(
            "InDesign automation preflight timed out. "
            f"InDesign was killed so the suite does not write {baseline_kind} render-error baselines.\n"
            f"stdout:\n{error.stdout or ''}\nstderr:\n{error.stderr or ''}"
        ) from error
    finally:
        script_path.unlink(missing_ok=True)

    if result.returncode != 0:
        kill_indesign()
        raise SystemExit(
            "InDesign automation preflight failed. "
            f"No {baseline_kind} baseline was written because InDesign is not scriptable right now.\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
