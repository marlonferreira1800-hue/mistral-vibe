from __future__ import annotations

from pathlib import Path

from vibe.core.paths import WORKTREES_DIR
from vibe.core.tools.models import ToolPermission
from vibe.core.tools.utils import is_path_within_workdir, resolve_file_tool_permission
from vibe.core.workspace import Workspace


def test_the_working_directory_is_authorised_without_being_a_project_root(
    tmp_path: Path,
) -> None:
    # What keeps an untrusted or unconfigured directory usable: the working
    # directory joins the set rather than being checked beside it.
    workspace = Workspace.for_session(tmp_path)

    assert workspace.authorized_roots == (tmp_path.resolve(),)


def test_project_roots_join_the_working_directory_without_duplicating_it(
    tmp_path: Path,
) -> None:
    other = tmp_path / "other"
    other.mkdir()

    workspace = Workspace.for_session(tmp_path, [tmp_path, other])

    assert workspace.authorized_roots == (tmp_path.resolve(), other.resolve())


def test_a_path_under_the_working_directory_is_refused_when_it_is_not_authorised(
    tmp_path: Path,
) -> None:
    # The point of the change. The boundary is the authorised set alone, so a
    # session whose directory is not in that set does not get to write there
    # merely by sitting in it. Before, anything under cwd was allowed outright.
    elsewhere = tmp_path / "elsewhere"
    elsewhere.mkdir()
    workspace = Workspace(cwd=tmp_path, authorized_roots=(elsewhere.resolve(),))

    assert not is_path_within_workdir("inside.txt", workspace=workspace)
    assert is_path_within_workdir(str(elsewhere / "ok.txt"), workspace=workspace)


def test_relative_paths_still_resolve_against_the_working_directory(
    tmp_path: Path,
) -> None:
    nested = tmp_path / "nested"
    nested.mkdir()
    workspace = Workspace.for_session(nested)

    assert is_path_within_workdir("file.txt", workspace=workspace)
    assert not is_path_within_workdir("../file.txt", workspace=workspace)


def test_worktrees_under_vibe_home_are_always_authorised(tmp_path: Path) -> None:
    workspace = Workspace.for_session(tmp_path)
    worktree_file = WORKTREES_DIR.path.resolve() / "bucket" / "worktree-1" / "src" / "main.py"

    assert workspace.allows(worktree_file)
    assert is_path_within_workdir(str(worktree_file), workspace=workspace)


def test_file_tool_permission_allows_vibe_worktree_without_outside_prompt(tmp_path: Path) -> None:
    workspace = Workspace.for_session(tmp_path)
    worktree_file = WORKTREES_DIR.path.resolve() / "bucket" / "my-worktree" / "file.txt"

    result = resolve_file_tool_permission(
        str(worktree_file),
        tool_name="read_file",
        allowlist=[],
        denylist=[],
        config_permission=ToolPermission.ALWAYS,
        sensitive_patterns=[],
        workspace=workspace,
    )
    # Inside workdir boundary (no OUTSIDE_DIRECTORY permission requested)
    assert result is None
