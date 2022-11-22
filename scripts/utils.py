import subprocess
from pathlib import Path


def toplevel():
    git_command = ["git", "rev-parse", "--show-toplevel"]
    git_output = subprocess.check_output(git_command, text=True)
    return Path(git_output.strip())
