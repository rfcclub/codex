from __future__ import annotations

from pathlib import Path

import pytest

from codex_shim import cli


def _executable(path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.touch()
    path.chmod(0o755)
    return path


def test_codex_binary_defaults_to_codexia_release_build(monkeypatch, tmp_path):
    shim_root = tmp_path / "codex-shim"
    binary = _executable(tmp_path / "codex-rs" / "target" / "release" / "codex")
    monkeypatch.setattr(cli, "PROJECT_ROOT", shim_root)
    monkeypatch.delenv("CODEX_SHIM_CODEX_BIN", raising=False)

    assert cli.codex_binary() == binary


def test_codex_binary_honors_explicit_override(monkeypatch, tmp_path):
    override = _executable(tmp_path / "custom-codex")
    monkeypatch.setenv("CODEX_SHIM_CODEX_BIN", str(override))

    assert cli.codex_binary() == override


def test_codex_binary_fails_closed_when_branch_binary_is_missing(monkeypatch, tmp_path):
    monkeypatch.setattr(cli, "PROJECT_ROOT", tmp_path / "codex-shim")
    monkeypatch.delenv("CODEX_SHIM_CODEX_BIN", raising=False)

    with pytest.raises(SystemExit, match="codexia binary is unavailable"):
        cli.codex_binary()


def test_exec_codex_replaces_process_with_codexia_binary(monkeypatch, tmp_path):
    binary = _executable(tmp_path / "codexia")
    monkeypatch.setenv("CODEX_SHIM_CODEX_BIN", str(binary))
    monkeypatch.setattr(cli, "_override_args", lambda settings_path, port: ["--config", "x=y"])
    calls = []

    def fake_execvpe(file, args, env):
        calls.append((file, args, env))
        raise RuntimeError("stop after capture")

    monkeypatch.setattr(cli.os, "execvpe", fake_execvpe)
    monkeypatch.setattr(cli.os, "name", "posix")

    with pytest.raises(RuntimeError, match="stop after capture"):
        cli.exec_codex(tmp_path / "models.json", 8787, ["--", "resume"])

    assert calls[0][0] == str(binary)
    assert calls[0][1] == [str(binary), "--config", "x=y", "resume"]
    assert calls[0][2]["NO_PROXY"].split(",")[-3:] == ["127.0.0.1", "localhost", "::1"]
