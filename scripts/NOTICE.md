# PLEASE NOTE

To be used as in

1. `python autogenerate_mail.py list_with_all_members.csv`
2. `python csv_to_sqlite.py list_with_all_members.csv ../../whatever_database.db`

Please check after step 1 if all mail addresses are indeed right. Common
mistakes the heuristic cannot detect are `von ...` which gets to `von_...@...`
while it should be actually just `...@...` without `von_`.
