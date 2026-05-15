"""Entry point for ``python -m hermes_cli``.

Allows users to run hermes without the console_script symlink resolving
correctly — useful when the venv's Scripts dir is not on PATH, or when
the symlink failed to create on Windows (issue #21465).

Usage:
    python -m hermes_cli [args]
    python -m hermes_cli doctor
    python -m hermes_cli dashboard
"""

from hermes_cli.main import main

if __name__ == "__main__":
    main()
