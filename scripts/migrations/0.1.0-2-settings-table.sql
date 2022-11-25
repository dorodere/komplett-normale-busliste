CREATE TABLE settings(
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    value,
    PRIMARY KEY (name)
);
INSERT INTO settings(name, description, value)
VALUES (
    "login-message",
    "Which custom message to display on the login page",
    "Falls du nicht reinkommen solltest, du hast dich vermutlich vertippt oder die falsche Email angegeben."
);
