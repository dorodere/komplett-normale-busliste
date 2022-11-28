#!/usr/bin/env python3
import sqlite3

from utils import toplevel


with open(toplevel() / "src/init_db.sql") as fh:
    INIT_DB_SCRIPT = fh.read()


def init_persons(cur, tablename="person"):
    # some hacky splitting work since we only want to create the person table here
    # needed for the import-from-csv mechanism

    tables = INIT_DB_SCRIPT.split("CREATE TABLE")
    for part in tables:
        lines = part.splitlines() or [""]
        if " person(" in lines[0]:
            person_table = "CREATE TABLE" + part

    with_modified_tablename = person_table.replace("person", tablename, 1)

    print(with_modified_tablename)
    cur.executescript(with_modified_tablename)


def init_database(filename: str = toplevel() / "testing-database.db"):
    conn = sqlite3.connect(filename)
    cur = conn.cursor()

    cur.executescript(INIT_DB_SCRIPT)

    cur.execute(
        """INSERT INTO person(prename, name, email, is_superuser, is_visible)
    VALUES ('John', 'Doe', 'john_doe@example.com', true, true)"""
    )

    conn.commit()
    conn.close()


if __name__ == "__main__":
    init_database()
