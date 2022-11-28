#!/usr/bin/env python3
# hacky, but it works (i think)
import argparse
import functools
import json
import os
import subprocess
from pathlib import Path

import migrate
from utils import toplevel


CARGO_METADATA_CMD = ["cargo", "metadata", "--format-version=1", "--no-deps"]


@functools.total_ordering
class SemVer:
    def __init__(self, source: str):
        numbers = source.split(".")
        self.major = int(numbers[0])
        self.minor = int(numbers[1])
        self.patch = int(numbers[2])

    def __eq__(self, other):
        return (
            self.major == other.major
            and self.minor == other.minor
            and self.patch == other.patch
        )

    def __lt__(self, other):
        return (
            self.major < other.major
            or self.minor < other.minor
            or self.patch < other.patch
        )


def version_from_cargo() -> str:
    os.chdir(toplevel())

    cargo_output = subprocess.check_output(CARGO_METADATA_CMD)
    raw_version = json.loads(cargo_output)["packages"][0]["version"]
    return SemVer(raw_version)


def find_migrations(before_version, after_version) -> list[str]:
    migration_dir = toplevel() / "scripts" / "migrations"

    def is_required(filename):
        version = SemVer(filename.split("-")[0])
        return before_version <= version and version < after_version

    all_migrations = list(os.listdir(migration_dir))
    all_migrations.sort()

    return list(
        map(
            lambda filename: migration_dir / filename,
            filter(is_required, all_migrations),
        )
    )


def upgrade(database: Path):
    before_version = version_from_cargo()

    for cmd in [["git", "pull"], ["cargo", "build", "--release"]]:
        subprocess.run(cmd)

    after_version = version_from_cargo()

    migrations_to_run = find_migrations(before_version, after_version)
    migrate.upgrade(database, migrations_to_run)


def parse_args():
    parser = argparse.ArgumentParser(
        prog="upgrader",
        description=(
            "Upgrades komplett-normale-busliste by pulling and applying all ",
            "needed registrations.",
        ),
        epilog="bottom text",
    )
    parser.add_argument("--database", required=True, type=Path)
    parser.add_argument("--repo", default=os.getcwd(), type=Path)
    return parser.parse_args()


def main():
    args = parse_args()

    os.chdir(args.repo)
    upgrade(args.database)


if __name__ == "__main__":
    main()
