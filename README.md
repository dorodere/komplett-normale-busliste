# komplett-normale-busliste

Digitalized version of what has once been done manually on paper. Probably doesn't scale very well,
but it fulfills its purpose.

### Tech used

This is a web service primarily written in [Rust], utilizing [Rocket] as web server implementation.
It's thought to be run in the LAN of an institution, not to be exposed to the outside world.

Some helper scripts are written in [Python].

[Rust]: https://www.rust-lang.org/
[Rocket]: https://rocket.rs/
[Python]: https://python.org/

### How to contribute

Thanks for considering doing so! You can take these steps:

1. Read the [code of conduct](./CODE_OF_CONDUCT.md).

2. Install [Git], [Rust] and [Python 3]. We're developing on Linux, though other platforms should
   work just fine as well.

3. Clone & navigate into the repository:

   ```
   git clone https://github.com/dorodere/komplett-normale-busliste.git
   cd komplett-normale-busliste
   ```

4. Copy the testing `RocketExample.toml` to `Rocket.toml` to have a development environment for
   Rocket set up:

   ```
   cp RocketExample.toml Rocket.toml
   ```

5. Initialize the database with some dummy data:

   ```
   python scripts/init_db.py
   ```

6. What follows is a bit ugly and will change for sure in the future, but you need to go into the
   source code and tell komplett-normale-busliste not to send any login emails, but rather just echo
   the login URL. You can do this with:

   ```
   git apply slightly-hacky-local-testing.patch
   ```

7. Compile, run and let's go!

   ```
   cargo run
   ```

8. Now open your web browser at `http://127.0.0.1:8008`. In the email field you can enter
   `john_doe@example.com`, hit enter, and click the link output in the console. You're logged in now
   and can do whatever you want! (in your local testing instance)

<details>
<summary markdown="span">Commands as one consistent block</summary>

```
git clone https://github.com/dorodere/komplett-normale-busliste.git
cd komplett-normale-busliste
cp RocketExample.toml Rocket.toml
python scripts/init_db.py
git apply slightly-hacky-local-testing.patch
cargo run
```

</details>

In case you're interested in general concepts, you can take a look at [the docs](./docs).

[Git]: https://git-scm.com/
[Rust]: https://doc.rust-lang.org/stable/book/ch01-01-installation.html
[Python 3]: https://wiki.python.org/moin/BeginnersGuide/Download

### How to use

First off: Don't. This is a very specific implementation for a very specific usecase, yours is
likely to be different from what you really need. Should you really want to host this specific
implementation, be aware of user privacy, how you expose what and where, and who exactly you give
access to your system.

Probably there will be some instructions here on how to self-host in the future. But for now, no.

### FAQ

#### Again, please

So, imagine you'd have an institution which provides a service for its members. This service is a
bus driving back and forth between two places on each weekend. But there's a slight problem with it.

You don't really know which kind of bus you'll need. Do you need the one with 1337 seats, which will
probably be a lot larger and more costly than others? Or do you need the one with 10 seats? There's
no way to tell except than asking every week again how many persons want to use the bus, or by using
heuristics.

In an attempt to solve this in a more comfortable way, you instead hand around a paper each week
which you get back on Wednesday, containing everyone who wants to use the bus. But this turns out to
be a mess: The paper is sometimes in a very bad state, contains unreadable handwriting and most
importantly costs you every week again one piece of paper.

Finally, you realise this needs be done in some better way. Probably with a website or app everyone
can log in internally, and you automatically receive a nice overview of how many seats you need, who
exactly wants to drive, and so on...

So you ask your friendly local confused gamer if they can do something about it. As such, here's the
solution!

<!--
vim: tw=100
-->
