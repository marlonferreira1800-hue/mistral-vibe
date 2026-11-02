from __future__ import annotations

import os
from pathlib import Path
from unittest.mock import MagicMock
import pytest

from mistralai_vibe_local_harness.vibe._storage import (
    _durable_replace,
    _windows_replace,
)


@pytest.mark.skipif(os.name != "nt", reason="Windows-specific MoveFileExW test")
def test_windows_replace_directory_nominal(tmp_path: Path) -> None:
    source = tmp_path / "staging"
    source.mkdir()
    (source / "manifest.json").write_text('{"v": 1}', encoding="utf-8")

    # In session storage generation promotion, target generation directory does not exist yet
    target = tmp_path / "gen_0001"

    _windows_replace(source, target)

    assert not source.exists()
    assert target.exists()
    assert (target / "manifest.json").read_text(encoding="utf-8") == '{"v": 1}'


@pytest.mark.skipif(os.name != "nt", reason="Windows-specific MoveFileExW test")
def test_windows_replace_file_nominal(tmp_path: Path) -> None:
    source = tmp_path / "temporary.json"
    source.write_text('{"current": 2}', encoding="utf-8")

    target = tmp_path / "current.json"
    target.write_text('{"current": 1}', encoding="utf-8")

    _windows_replace(source, target)

    assert not source.exists()
    assert target.exists()
    assert target.read_text(encoding="utf-8") == '{"current": 2}'


def test_windows_replace_retries_transient_access_denied(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    source = tmp_path / "source"
    target = tmp_path / "target"

    sleeps: list[float] = []

    mock_move = MagicMock()
    # Fails twice with ERROR_ACCESS_DENIED (5), succeeds on 3rd attempt
    mock_move.side_effect = [False, False, True]

    mock_get_last_error = MagicMock()
    mock_get_last_error.side_effect = [5, 5]

    class MockWinDLL:
        def __init__(self, name: str, use_last_error: bool = False) -> None:
            self.MoveFileExW = mock_move

    monkeypatch.setattr("ctypes.WinDLL", MockWinDLL)
    monkeypatch.setattr("ctypes.get_last_error", mock_get_last_error)
    monkeypatch.setattr("time.sleep", lambda s: sleeps.append(s))

    _windows_replace(source, target)

    assert mock_move.call_count == 3
    assert len(sleeps) == 2
    assert sleeps[0] == pytest.approx(0.05)
    assert sleeps[1] == pytest.approx(0.10)


def test_windows_replace_aborts_immediately_on_non_transient_error(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    source = tmp_path / "source"
    target = tmp_path / "target"

    sleeps: list[float] = []
    mock_move = MagicMock(return_value=False)
    # ERROR_FILE_NOT_FOUND (2) is not a transient lock error
    mock_get_last_error = MagicMock(return_value=2)

    class MockWinDLL:
        def __init__(self, name: str, use_last_error: bool = False) -> None:
            self.MoveFileExW = mock_move

    monkeypatch.setattr("ctypes.WinDLL", MockWinDLL)
    monkeypatch.setattr("ctypes.get_last_error", mock_get_last_error)
    monkeypatch.setattr("time.sleep", lambda s: sleeps.append(s))

    with pytest.raises(OSError) as exc_info:
        _windows_replace(source, target)

    # Must only attempt once and break immediately without sleeping
    assert mock_move.call_count == 1
    assert len(sleeps) == 0
    assert exc_info.value.winerror == 2


def test_windows_replace_exhausts_retries_on_persistent_sharing_violation(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    source = tmp_path / "source"
    target = tmp_path / "target"

    sleeps: list[float] = []
    mock_move = MagicMock(return_value=False)
    # ERROR_SHARING_VIOLATION (32) persistently locked
    mock_get_last_error = MagicMock(return_value=32)

    class MockWinDLL:
        def __init__(self, name: str, use_last_error: bool = False) -> None:
            self.MoveFileExW = mock_move

    monkeypatch.setattr("ctypes.WinDLL", MockWinDLL)
    monkeypatch.setattr("ctypes.get_last_error", mock_get_last_error)
    monkeypatch.setattr("time.sleep", lambda s: sleeps.append(s))

    with pytest.raises(OSError) as exc_info:
        _windows_replace(source, target)

    # Must attempt all 6 times before surfacing the error
    assert mock_move.call_count == 6
    assert len(sleeps) == 6
    assert exc_info.value.winerror == 32
