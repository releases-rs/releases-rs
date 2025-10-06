# Rust Changelogs

Generates the pages at <https://releases.rs/>. A GitHub workflow regenerates
the pages periodically.

## Building

Note: requires [hugo](https://gohugo.io/) extended 0.146.7 or higher.

```shell
git clone --recurse-submodules git@github.com:releases-rs/releases-rs
cd releases-rs
cargo run
```

Note: if the GitHub API rate limit is reached, a [personal access token (classic)](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#types-of-personal-access-tokens)
can be provided via the `GITHUB_TOKEN` env.

When done you will have your generated pages in `hugo/rust-changelogs/public`.

### Serving Locally

```shell
cd hugo/rust-changelogs
hugo serve --theme hugo-book
```

The site will be available at <http://localhost:1313>
