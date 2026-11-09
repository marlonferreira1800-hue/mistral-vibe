"""POSIX drive paths handed by an ACP host must not double on Windows.

A host (e.g. the T3 harness) spells paths in Git Bash form (``/c/Users/...``).
win32 resolution anchors such a string to the process's current drive, so
``/c/Users/x`` seen from drive F: resolves to ``F:\\c\\Users\\x`` and a file
tool writes outside the worktree. The sandbox helper folds the drive segment
before resolving (T177).
"""

from __future__ import annotations

from pathlib import Path
import sys

import pytest

sandbox_helper = pytest.importorskip(
    "mistralai_vibe_local_harness.vibe._sandbox_helper"
)


@pytest.fixture
def on_windows(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(sys, "platform", "win32")


@pytest.mark.parametrize(
    ("raw", "expected"),
    [
        ("/c/Users/acmedev/repo", "C:/Users/acmedev/repo"),
        ("/f/Projects/ai-routing", "F:/Projects/ai-routing"),
        ("/c", "C:/"),
        ("/c\\Users\\acmedev", "C:/Users/acmedev"),
        (r"C:\Users\acmedev\repo", r"C:\Users\acmedev\repo"),
        ("C:/already/windows", "C:/already/windows"),
        ("/Users/acmedev", "/Users/acmedev"),
        ("relative/path", "relative/path"),
        ("//server/share", "//server/share"),
    ],
)
@pytest.mark.usefixtures("on_windows")
def test_normalize_windows_path_folds_msys_drive_paths(raw: str, expected: str) -> None:
    assert sandbox_helper.normalize_windows_path(raw) == expected


def test_normalize_windows_path_is_a_no_op_off_windows(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(sys, "platform", "linux")
    assert sandbox_helper.normalize_windows_path("/c/Users/acmedev") == (
        "/c/Users/acmedev"
    )


def _msys(path: Path) -> str:
    """Spell a Windows path the way a POSIX host hands it: ``/c/Users/...``."""
    posix = path.as_posix()
    return f"/{path.drive[0].lower()}{posix[2:]}"


def _policy(cwd: Path, roots: list[str]) -> dict:
    return {
        "cwd": str(cwd),
        "roots": roots,
        "roots_are_a_boundary": True,
        "authorized": None,
        "authorized_as_written": False,
    }


@pytest.mark.skipif(sys.platform != "win32", reason="drive-letter resolution is win32")
def test_resolve_path_folds_posix_drive_path(tmp_path: Path) -> None:
    probe = tmp_path / "probe.txt"
    probe.write_text("probe", encoding="utf-8")

    resolved = sandbox_helper.resolve_path(
        _msys(probe),
        cwd=str(tmp_path),
        roots=[str(tmp_path)],
        roots_are_a_boundary=True,
    )

    assert resolved == probe.resolve()


@pytest.mark.skipif(sys.platform != "win32", reason="drive-letter resolution is win32")
def test_run_file_tool_writes_and_reads_posix_drive_path_on_its_drive(
    tmp_path: Path,
) -> None:
    target = tmp_path / "written.txt"
    write_request = {
        "tool": "write_file",
        "arguments": {"path": _msys(target), "content": "hello"},
        "policy": _policy(tmp_path, [str(tmp_path)]),
    }
    response = sandbox_helper.run_file_tool(write_request)

    assert target.read_text(encoding="utf-8") == "hello"
    assert response["result"]["path"] == str(target.resolve())

    read_request = {
        "tool": "read_file",
        "arguments": {"path": _msys(target), "offset": 0, "limit": None},
        "policy": _policy(tmp_path, [str(tmp_path)]),
    }
    response = sandbox_helper.run_file_tool(read_request)

    assert response["result"]["content"] == "hello"


@pytest.mark.skipif(sys.platform != "win32", reason="drive-letter resolution is win32")
def test_run_file_tool_keeps_posix_drive_path_inside_the_workspace(
    tmp_path: Path,
) -> None:
    outside = tmp_path.parent / "t177-outside.txt"
    request = {
        "tool": "write_file",
        "arguments": {"path": _msys(outside), "content": "no"},
        "policy": _policy(tmp_path, [str(tmp_path)]),
    }
    with pytest.raises(ValueError, match="outside the workspace"):
        sandbox_helper.run_file_tool(request)
    assert not outside.exists()
