# The authentication flow

The authentication flow is quite simple, but relies on the security of the email
provider.

## Abstract procedure

- On the first visit of the user on the index site, they just get presented a
	login form with the only field being their email address.
- Upon sending this form to the server, this email address is searched in the
	database for the associated person. The server then generates a random token
	for that user, notes it down in the database, and sends a link with that token
	to the associated email address.
- When clicking on that link, this token gets sent to the server. The server
	generates a JWT cookie and sends it back to the user, alongside with the
	dashboard.

## Implementation

- The first visit on the site is marked down by a simple `GET /`, without any
	cookies. Server sees that no authenticating cookie with a JWT is present, and
	returns a simple login page.
- As the server gets the email address form back on `POST /`, it
  1. Searches for the person with that email in the database
	2. Generates a random 128 byte token, URL-safe base64 encoded
	3. Sends the token with `protocol://domain.toplevel/login/` + person id as
		 `GET` parameter prefixed per email to the associated email address
	4. Hashes that token with a random salt and stores it in the database
	5. Replies with a redirect to the login page, noting that a login link per
		 emali was sent
- When the user clicks on the link which was sent per email, the server finds
	the person associated with the email, hashes the token, and compares it with
	the entry in the database. Then it replies with a freshly generated JWT, valid
	for 30 days, and a dashboard.

Or, alternatively, if you like ASCII art more:
```text
  user (browser)                                     server
        |                                              |
        |         "GET /"                              |
        | -------------------------------------------> | Sees that no JWT token is present
        |                                              |
        |                           Login form         |
        | <------------------------------------------- |
        |                                              |
        |                                              |
        |         "POST /"                             |
        | -------------------------------------------> | 1. Searches for person by email
        |                                              | 2. Generates token
        |                  Flash redirect to /         | 3. Sends email w/ token + person id in link
        | <------------------------------------------- | 4. Hashes token and stores it in DB
        |                                              | 
        |                                              |
        |         "GET /login/<token>?<person_id>"     |
        | -------------------------------------------> | 1. Searches for token hash by person id
        |                                              | 2. Check if the token didn't expire yet
        |            Dashboard with JWT cookie         | 3. Verifies token by client with token hash in DB
        | <------------------------------------------- | 4. Generates JWT and returns it in a cookie
        |                                              |
        |                                              |
        |                                              |

                              ...

        |         "GET /" w/ JWT cookie                |
        | -------------------------------------------> | JWT token is present & valid
        |                                              |
        |                            Dashboard         |
        | <------------------------------------------- |
        |                                              |
```

