CREATE TABLE person(
    person_id INTEGER,
    prename TEXT NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    token TEXT,
    token_expiration INTEGER,
    is_superuser BOOLEAN NOT NULL,
    UNIQUE(email),
    PRIMARY KEY (person_id AUTOINCREMENT)
)
CREATE TABLE drive(
    drive_id INTEGER,
    drivedate DATE NOT NULL,
    UNIQUE(drivedate),
    PRIMARY KEY (drive_id AUTOINCREMENT)
)
CREATE TABLE registration(
    id INTEGER,
    person_id INTEGER NOT NULL,
    drive_id INTEGER NOT NULL,
    registered BOOLEAN NOT NULL,
    UNIQUE(person_id, drive_id),
    FOREIGN KEY (person_id) REFERENCES person(person_id)
            ON DELETE CASCADE
            ON UPDATE CASCADE,
    FOREIGN KEY (drive_id) REFERENCES drive(drive_id)
            ON DELETE CASCADE
            ON UPDATE CASCADE,
    PRIMARY KEY (id AUTOINCREMENT) 
)
