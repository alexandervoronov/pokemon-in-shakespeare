# Pokémon teller in Shakespeare English

A web service that provides brief descriptions of Pokémon in Shakespeare-style English.

For example,

```
> curl http://localhost:5000/pokemon/blastoise
{
  "name": "blastoise",
  "description": "Blastoise hath water spouts yond protrude from its shell. The water spouts art very accurate. They can shoot bullets of water with enow accuracy to strike exsufflicate cans from a distance of ov'r 160 feet."
}
```

## API

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

## Build and run

You can build Pokémon teller as a native binary using [Cargo](https://doc.rust-lang.org/cargo/) or
as a container image with a Dockerfile provided in the repo.

### Building with Cargo

```
git clone https://github.com/alexandervoronov/pokemon-in-shakespeare
cd pokemon-in-shakespeare
cargo run --release
```

### Building with Docker

```
git clone https://github.com/alexandervoronov/pokemon-in-shakespeare
cd pokemon-in-shakespeare
docker build --tag pokemon-in-shakespeare .
docker run -p 5000:5000 pokemon-in-shakespeare
```

## Limitations

- The service used a fixed port 5000, which can be worked around by using a docker image and
  remapping the port
- Shakespeare translator API has quite strict request rate limit (5 per hour, 60 per day), so
  after a few requests the Pokémon teller will switch to modern English

## Implementation details

Pokémon teller is written in Rust and relies on [Poké API](https://pokeapi.co/) and
[Shakespeare translator API](https://funtranslations.com/api/shakespeare) for the content.
Successful responses from the content services are cached, therefore repeated requests are served
faster.

### Potential improvements

- A bit of command-line configuration. First candidates are the network port and a switch for
  passing through the `TOO_MANY_REQUESTS` error from _Shakespeare translator_ instead of returning
  an unshakespearised test.
- Configurable logging verbosity. Current logging is just printing to _cerr_.
- A bit more attention to cleanup/formatting of the Pokémon descriptions. _Poké API_ often returns
  double spaces and random Unicode characters that look weird specially when we exceed the
  _Shakespeare translator_ request quota.
- Spending some time on breaking the project into multiple files. The amount of code currently is
  a bit over “fits nicely into a single file” but yet didn’t exceed “already unmanageable in a
  single file”.
- Seeing if switch of the docker image base to Alpine instead of Ubuntu reduces the image size
  significantly.
- Better separation of caching from the main logic. It's currently somewhat entangled mostly due
  to the need of handling `TOO_MANY_REQUESTS` error from _Shakespeare translator_. Another issue
  with the current cache implementation is that it doesn't have any size controls on the number of
  entries or the size of an entry. So with serving more and more requests it'll eventually
  cache 'em all.
- Load testing and cache adjustments/rework. Without load testing it's hard to tell how good the
  current caching is and also makes it almost pointless to try something different.
