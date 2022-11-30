CREATE TABLE IF NOT EXISTS person(
    person_id INTEGER,
    prename TEXT NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    token TEXT,
    token_expiration INTEGER,
    is_superuser BOOLEAN NOT NULL,
    is_visible BOOLEAN NOT NULL,
    UNIQUE(email),
    PRIMARY KEY (person_id AUTOINCREMENT)
);
CREATE TABLE IF NOT EXISTS drive(
    drive_id INTEGER,
    drivedate DATE NOT NULL,
    deadline DATETIME,
    UNIQUE(drivedate),
    PRIMARY KEY (drive_id AUTOINCREMENT)
);
CREATE TABLE IF NOT EXISTS registration(
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
);
CREATE TABLE settings(
    name TEXT NOT NULL,
    value,
    PRIMARY KEY (name)
);
INSERT INTO settings(name, value)
VALUES (
    "login-message",
    "Falls du nicht reinkommen solltest, du hast dich vermutlich vertippt oder die falsche Email angegeben."
), (
    "default-deadline",
    2
), (
    "default-registration-cap",
    50
);
