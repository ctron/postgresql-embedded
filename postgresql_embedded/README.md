# PostgreSQL Embedded

[![ci](https://github.com/theseus-rs/postgresql-embedded/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/theseus-rs/postgresql-embedded/actions/workflows/ci.yml)
[![Documentation](https://docs.rs/postgresql_embedded/badge.svg)](https://docs.rs/postgresql_embedded)
[![Code Coverage](https://codecov.io/gh/theseus-rs/postgresql-embedded/branch/main/graph/badge.svg)](https://codecov.io/gh/theseus-rs/postgresql-embedded)
[![Benchmarks](https://img.shields.io/badge/%F0%9F%90%B0_bencher-enabled-6ec241)](https://bencher.dev/perf/theseus-rs-postgresql-embedded)
[![Latest version](https://img.shields.io/crates/v/postgresql_embedded.svg)](https://crates.io/crates/postgresql_embedded)
[![License](https://img.shields.io/crates/l/postgresql_embedded)](https://github.com/theseus-rs/postgresql-embedded/tree/main/postgresql_embedded#license)
[![Semantic Versioning](https://img.shields.io/badge/%E2%9A%99%EF%B8%8F_SemVer-2.0.0-blue)](https://semver.org/spec/v2.0.0.html)

Install and run a PostgreSQL database locally on Linux, MacOS or Windows. PostgreSQL can be
bundled with your application, or downloaded on demand.

## Examples

### Asynchronous API

```rust
use postgresql_embedded::{PostgreSQL, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut postgresql = PostgreSQL::default();
    postgresql.setup().await?;
    postgresql.start().await?;

    let database_name = "test";
    postgresql.create_database(database_name).await?;
    postgresql.database_exists(database_name).await?;
    postgresql.drop_database(database_name).await?;

    postgresql.stop().await
}
```

### Synchronous API

```rust
use postgresql_embedded::Result;
use postgresql_embedded::blocking::PostgreSQL;

fn main() -> Result<()> {
    let mut postgresql = PostgreSQL::default();
    postgresql.setup()?;
    postgresql.start()?;

    let database_name = "test";
    postgresql.create_database(database_name)?;
    postgresql.database_exists(database_name)?;
    postgresql.drop_database(database_name)?;

    postgresql.stop()
}
```

## Information

During the build process, when the `bundled` feature is enabled, the PostgreSQL binaries are
downloaded and included in the resulting binary. The version of the PostgreSQL binaries is
determined by the `POSTGRESQL_VERSION` environment variable. If the `POSTGRESQL_VERSION`
environment variable is not set, then `postgresql_archive::LATEST` will be used to determine the
version of the PostgreSQL binaries to download.

When downloading the PostgreSQL binaries, either during build, or at runtime, the `GITHUB_TOKEN`
environment variable can be set to a GitHub personal access token to increase the rate limit for
downloading the PostgreSQL binaries. The `GITHUB_TOKEN` environment variable is not required.

At runtime, the PostgreSQL binaries are cached by default in the following directories:

- Unix: `$HOME/.theseus/postgresql`
- Windows: `%USERPROFILE%\.theseus\postgresql`

## Feature flags

postgresql_embedded uses feature flags to address compile time and binary size
uses.

The following features are available:

| Name         | Description                                              | Default? |
|--------------|----------------------------------------------------------|----------|
| `bundled`    | Bundles the PostgreSQL archive into the resulting binary | No       |
| `blocking`   | Enables the blocking API; requires `tokio`               | No       |
| `native-tls` | Enables native-tls support                               | No       |
| `rustls-tls` | Enables rustls-tls support                               | Yes      |
| `tokio`      | Enables using tokio for async                            | No       |

## Safety

This crate uses `#![forbid(unsafe_code)]` to ensure everything is implemented in 100% safe Rust.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Notes

Uses PostgreSQL binaries from [theseus-rs/postgresql-binaries](https://github.com/theseus-rs/postgresql_binaries).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
