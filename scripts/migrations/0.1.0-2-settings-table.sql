CREATE TABLE settings(
    name TEXT NOT NULL,
    value,
    PRIMARY KEY (name)
);
INSERT INTO settings(name, value)
VALUES (
    "login-message",
    "Falls du nicht reinkommen solltest, du hast dich vermutlich vertippt oder die falsche Email angegeben."
);
