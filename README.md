= Pokémon teller in Shakespeare English =

A web service that provides brief description of Pokémon in Shakespeare-style English.

For example,

```
> curl http://localhost:5000/pokemon/blastoise
{
  "name": "blastoise",
  "description": "Blastoise hath water spouts yond protrude from its shell. The water spouts art very accurate. They can shoot bullets of water with enow accuracy to strike exsufflicate cans from a distance of ov'r 160 feet."
}
```

== API ==

Query:

```
http://<server_address>:5000/pokemon/<pokemon name>
```

Response:

```
// Content-Type: application/json; charset=UTF-8
{
    "name" : String,
    "description" : String
}
```

== Build and run ==
You can build Pokémon teller as a native binary using [Cargo](https://doc.rust-lang.org/cargo/) or
as a container image with a Dockerfile provided in the repo.

=== Building with Cargo ===

```
git clone https://github.com/alexandervoronov/pokemon-in-shakespeare
cd pokemon-in-shakespeare
cargo run --release
```

=== Building with Docker ===

```
git clone https://github.com/alexandervoronov/pokemon-in-shakespeare
cd pokemon-in-shakespeare
docker build --tag pokemon-in-shakespeare .
docker run -p 5000:5000 pokemon-in-shakespeare
```

== Implementation details ==
Pokémon teller is written in Rust and relies on [Poké API](https://pokeapi.co/) and
[Shakespeare translator API](https://funtranslations.com/api/shakespeare) for the content.
Successful responses from the content services are cached, therefore repeated requests are served
faster.

== Limitations ==

- The service used a fixed port 5000, which can be worked around by using a docker image and
  remapping the port
- Shakespeare translator API has quite strict request rate limit (5 per hour, 60 per day), so
  after a few requests the Pokémon teller will switch to modern English
