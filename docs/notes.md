# Notes

Just some random notes.

## User authentication concept

On the login page, the user can enter their email. If this email is present in
the database, it is messaged with a link with a login token in as a GET
parameter. Else, the user is displayed that their email is invalid. The token is
valid for 30 minutes.

After clicking on the link with the token as a GET parameter, the user gets
redirected to the dashboard with a GET request, while the token gets removed
from the URL and another generated token gets sent as a session cookie, valid
for two hours.
