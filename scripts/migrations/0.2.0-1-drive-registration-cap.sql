ALTER TABLE drive
ADD COLUMN registration_cap INTEGER DEFAULT null;

INSERT INTO settings(name, value)
VALUES (
    "default-registration-cap",
    50
);
