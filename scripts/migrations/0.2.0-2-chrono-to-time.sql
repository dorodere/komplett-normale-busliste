UPDATE drive
SET
    drivedate = drivedate || " 00:00:00.0Z",
    deadline = deadline || ".0Z";
