# Rust Changelogs

Generates the pages at <https://releases.rs/>. A GitHub workflow regenerates
the pages periodically.

## Building

Note: this requires [hugo](https://gohugo.io/)

```shell
git clone --recurse-submodules git@github.com:glebpom/rust-changelogs
cd rust-changelogs
cargo run
```

When done you will have your generated pages in `hugo/rust-changelogs/public`.

### Serving Locally

```shell
cd hugo/rust-changelogs
hugo serve --theme hugo-book
```

The site will be available at <http://localhost:1313>
