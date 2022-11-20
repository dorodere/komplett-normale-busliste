#!/usr/bin/env python3
import csv
import sqlite3
import sys

from init_db import init_persons


def quit(cur, conn, code=1):
    cur.execute("DROP TABLE import_person")
    conn.commit()
    conn.close()
    sys.exit(code)


def main(argv):
    source = argv[1]
    output = argv[2]

    conn = sqlite3.connect(output)
    cur = conn.cursor()

    # set up import table to run queries on
    init_persons(cur, tablename="import_person")
    with open(source) as fh:
        reader = csv.reader(fh)

        for row in reader:
            name = row[0].strip()
            prename = row[1].strip()
            email = row[2].strip()
            try:
                cur.execute(
                    "INSERT INTO import_person(prename, name, email, is_superuser) VALUES (?, ?, ?, 0)",
                    (
                        prename,
                        name,
                        email,
                    ),
                )
            except sqlite3.IntegrityError:
                print(
                    f"warning: failed to insert '{name}', '{prename}', does the name exist twice?",
                    file=sys.stderr,
                )

    # check for email conflicts
    conflicts = cur.execute(
        """
        SELECT person.name, person.prename, person.email, import_person.email
        FROM person
        LEFT OUTER JOIN import_person ON person.name == import_person.name
                                     AND person.prename == import_person.prename
        WHERE person.email != import_person.email
        """
    ).fetchall()

    if conflicts:
        print(
            "warning: there are email conflicts in old vs. new, old values are to be deleted!",
            file=sys.stderr,
        )
        for name, prename, old_email, new_email in conflicts:
            print(
                f"\t{name}, {prename}: old: '{old_email}', new: '{new_email}'",
                file=sys.stderr,
            )

    # stage 1: insert new persons
    cur.execute(
        """
        INSERT INTO person (prename, name, email, is_superuser)
            SELECT import_person.prename, import_person.name, import_person.email, false
            FROM import_person
            LEFT OUTER JOIN person ON person.email == import_person.email
            WHERE person.email IS NULL
        """
    )

    # stage 2: delete old ones
    cur.execute(
        """
        DELETE FROM person
        WHERE person.email IN (
            SELECT person.email
            FROM person
            LEFT OUTER JOIN import_person ON person.email == import_person.email
            WHERE NOT person.is_superuser AND import_person.email IS NULL
        )
        """
    )

    print(
        "done, don't forget to manually check the database for duplicates and the like"
    )

    quit(cur, conn, code=0)


if __name__ == "__main__":
    main(sys.argv)
