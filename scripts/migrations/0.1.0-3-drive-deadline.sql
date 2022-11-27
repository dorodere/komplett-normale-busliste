ALTER TABLE drive
ADD COLUMN deadline DATETIME DEFAULT null;

INSERT INTO settings(name, value)
VALUES (
    "default-deadline",
    2
);
