#!/usr/bin/env python
import csv
import sys
import tempfile


def remove_umlaute(source):
    return source.replace("ü", "ue").replace("ä", "ae").replace("ö", "oe")


def main(argv):
    subject = argv[1]

    with tempfile.TemporaryFile(mode="w+") as target:
        with open(subject) as fh:
            writer = csv.writer(target)
            reader = csv.reader(fh)

            for row in reader:
                name = row[0].strip()
                prename = row[1].strip()
                email = f"{name.lower().replace(' ', '_')}_{prename.lower().replace(' ', '_')}@example.com"
                email = remove_umlaute(email)

                writer.writerow([name, prename, email])

        target.flush()
        target.seek(0)

        with open(subject, "w") as fh:
            fh.write(target.read())


if __name__ == "__main__":
    main(sys.argv)
