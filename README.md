<h1 align="center">async-session</h1>
<div align="center">
  <strong>
    Async session support with pluggable middleware
  </strong>
</div>

<br />

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/async-session">
    <img src="https://img.shields.io/crates/v/async-session.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/async-session">
    <img src="https://img.shields.io/crates/d/async-session.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/async-session">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://docs.rs/async-session">
      API Docs
    </a>
    <span> | </span>
    <a href="https://github.com/http-rs/async-session/releases">
      Releases
    </a>
    <span> | </span>
    <a href="https://github.com/http-rs/async-session/blob/main/.github/CONTRIBUTING.md">
      Contributing
    </a>
  </h3>
</div>

## Available session stores

* [async-sqlx-session](https://crates.io/crates/async-sqlx-session) postgres & sqlite
* [async-redis-session](https://crates.io/crates/async-redis-session)
* [async-mongodb-session](https://crates.io/crates/async-mongodb-session)

## Framework implementations

* [`tide::sessions`](https://docs.rs/tide/latest/tide/sessions/index.html)
* [warp-sessions](https://docs.rs/warp-sessions/latest/warp_sessions/)
* [trillium-sessions](https://docs.trillium.rs/trillium_sessions)

## Safety
This crate uses ``#![deny(unsafe_code)]`` to ensure everything is implemented in
100% Safe Rust.

## Contributing
Want to join us? Check out our ["Contributing" guide][contributing] and take a
look at some of these issues:

- [Issues labeled "good first issue"][good-first-issue]
- [Issues labeled "help wanted"][help-wanted]

[contributing]: https://github.com/http-rs/async-session/blob/main/.github/CONTRIBUTING.md
[good-first-issue]: https://github.com/http-rs/async-session/labels/good%20first%20issue
[help-wanted]: https://github.com/http-rs/async-session/labels/help%20wanted

## Acknowledgements

This work is based on the work initiated by
[@chrisdickinson](https://github.com/chrisdickinson) in
[tide#266](https://github.com/http-rs/tide/pull/266).

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br/>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
