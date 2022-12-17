#!/usr/bin/env python3
import argparse
import shutil
import sqlite3
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

from utils import toplevel


def upgrade(database: Path, migrations):
    # copy the database temporarily to roll back if necessary
    with TemporaryDirectory() as backupdir:
        shutil.copyfile(database, Path(backupdir) / "backup.db")

        try:
            conn = sqlite3.connect(database)
            cur = conn.cursor()

            for script_path in migrations:
                with open(script_path) as fh:
                    script = fh.read()

                cur.executescript(script)

            conn.commit()
            conn.close()

        except Exception as err:
            print("### Error occurred, rolling database back!", file=sys.stderr)
            shutil.copyfile(Path(backupdir) / "backup.db", database)

            print(f"### Current script: {script_path}\n", file=sys.stderr)

            raise err


def parse_args():
    parser = argparse.ArgumentParser(
        prog="database migration helper",
        description=(
            "Migrates SQLite databases by applying the given SQL scripts, "
            "rolling back if any errors occur"
        ),
        epilog="bottom text",
    )
    parser.add_argument(
        "--database",
        default=str(toplevel() / "crates" / "backend" / "testing-database.db"),
        type=Path,
    )
    parser.add_argument("migration_scripts", nargs="+", type=Path)
    return parser.parse_args()


def main():
    args = parse_args()
    upgrade(args.database, args.migration_scripts)


if __name__ == "__main__":
    main()
