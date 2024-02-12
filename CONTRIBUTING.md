# Contribution

Thanks for contributing to the trunk toolbox! Read on to learn more.

- [Overview](#overview)
- [Development](#development)
- [Testing](#testing)
- [Guidelines](#guidelines)
- [Docs](https://docs.trunk.io)

## Overview

Trunk toolbox is a place for rules that transcend particular languages and are relevant to any code in a repo.

## Development

The trunk.yaml in this repo has been modified to run your local iteration of toolbox as you are building. This is managed through the `trunk-latest` script. Effectively as you run `cargo build` or `cargo build release` the script will pick up the last built binary and use that.

If no local binary has been built then the pinned version in the trunk.yaml will be used.

## Testing

`cargo test` will execute the unit and integration tests for the repo

## Guidelines

Please follow the guidelines below when contributing:

- After defining a rule, please add it to [`README.md`](README.md).
- If you run into any problems while defining a rule, feel free to reach out on our
  [Slack](https://slack.trunk.io/).
