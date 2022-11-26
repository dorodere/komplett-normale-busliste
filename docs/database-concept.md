# The database concept

Phew, this is my first time I'm working with a real database. But I'll try my
best working out an usable concept for that one.

Generally, I think that three tables should be in that database: `person`,
`drivedate` and `registration`.

## Class diagrams

### `person`

`person` should contain the prename, name and email of the persons who drive at
all in the bus. Prename and name are needed to identify the persons at all when
a person in the bus controls which persons are in the bus and which are not,
email is needed for login.

Token and expiration timepoint are needed for authentication, see `authflow.md`.
The email needs to be unique as that one is being used for authentication.
Expiration timepoint is a UTC UNIX timestamp, measured in seconds.

Also a field is needed to check on login whether to give the client on login a
normal user token or a superuser token, which allows them to register more
drives dates as well as seeing all persons that registered.

Additionally, unrelated to whether a person is a superuser, it's also possible
that a person wants to drive with the bus anyways. So that's two columns.

As a such, I think the class diagram of `person` should look like this:

```text
+--------------------------------------+
|                person                |
+--------------------------------------+
|   person_id INTEGER (primary key)    |
|             prename TEXT             |
|              name TEXT               |
|              email TEXT            --|--- unique
|              token TEXT              |
|       token_expiration INTEGER       |
|         is_superuser BOOLEAN         |
|          is_visible BOOLEAN          |
+--------------------------------------+
```

### `drive`

The table `drive` contains the dates on which the bus drives at all. This
is done in order to check if a registration in `registraton` can be valid at
all: There is no point in registering to drive if the bus doesn't drive on that
day. Since it's task is to just hold on which dates a registration is okay and
when its final registration opportunity is, it only consists out of three
columns, date, id and deadline:

```text
+--------------------------------------+
|                drive                 |
+--------------------------------------+
|    drive_id INTEGER (primary key)    |
|            drivedate DATE          --|--- unique
|           deadline INTEGER           |
+--------------------------------------+
```

### `registration`

`registration` contains all entries when a person registered to use the bus. So
it needs a reference to the person which the entry belongs to, a date to
determine if the bus usage was twenty years ago or in a week, and boolean if the
person registered. Checking if the person actually drove is out of scope of the
application.

Since it would make no sense if there are two different entries for the same
person and date, they must be unique in union. Also, since the date actually has
to refer to a row in `drive`, it can be marked as foreign key.

To conclude, the class diagram might be this:

```text
+--------------------------------------+
|             registration             |
+--------------------------------------+
|       id INTEGER (primary key)       |
|   person_id INTEGER (foreign key)  --|--+ unique
|    drive_id INTEGER (foreign key)  --|-/
|          registered BOOLEAN          |
+--------------------------------------+
```

### `settings`

This is not really related to the main functionality of the application, but
rather just a key-value table. There's some instance-specific options which
still need to be configurable by the end-user though, and this table holds
exactly those, being mostly static.

```
+--------------------------------------+
|               settings               |
+--------------------------------------+
|       name TEXT (primary key)        |
|           value (untyped)            |
+--------------------------------------+
```

## Database layout

The class diagrams are easily translatable to a [database layout].

[database layout]: ../src/init_db.sql

<!--
vim: et sts=0 ts=2 sw=2
-->
