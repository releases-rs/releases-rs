# Rust Changelogs

Generates the pages at <https://releases.rs/>. A GitHub workflow regenerates
the pages periodically.

## Building

Note: this requires [hugo](https://gohugo.io/)

```shell
sh $ git clone --recurse-submodules git@github.com:glebpom/rust-changelogs
sh $ cd rust-changelogs
sh $ cargo run
```

This will take a while, but when done you will have your generated pages in `hugo/rust-changelogs/public`.

### Serving Locally

```shell
sh $ cd hugo/rust-changelogs
sh $ hugo serve --theme hugo-book
```

The site will be available at <http://localhost:1313>
