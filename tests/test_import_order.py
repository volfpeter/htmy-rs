import subprocess
import sys


def test_renderer_selection_import_order() -> None:
    """Importing `htmy_rs` before `htmy` must not break the default renderer selection in `htmy`."""
    code = "\n".join(
        [
            "import htmy_rs.renderer, htmy",
            "assert htmy.Renderer is htmy_rs.renderer.Renderer",
            "assert htmy.renderer.Renderer is htmy_rs.renderer.Renderer",
        ]
    )
    result = subprocess.run(
        [sys.executable, "-c", code],
        capture_output=True,
        text=True,
        timeout=5,
    )
    assert result.returncode == 0, result.stderr
