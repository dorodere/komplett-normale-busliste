ALTER TABLE drive
ADD COLUMN deadline INTEGER DEFAULT null;

INSERT INTO settings(name, value)
VALUES (
    "default-deadline",
    2
);
