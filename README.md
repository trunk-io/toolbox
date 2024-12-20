<!-- trunk-ignore-all(trunk-toolbox) -->
<!-- trunk-ignore-all(markdownlint/MD024) -->

<!-- trunk-ignore(markdownlint/MD041) -->
<p align="center">
  <a href="https://marketplace.visualstudio.com/items?itemName=Trunk.io">
    <img src="https://img.shields.io/visual-studio-marketplace/i/Trunk.io?logo=visualstudiocode"/>
  </a>
  <a href="https://slack.trunk.io">
    <img src="https://img.shields.io/badge/slack-slack.trunk.io-blue?logo=slack"/>
  </a>
  <a href="https://docs.trunk.io">
    <img src="https://img.shields.io/badge/docs.trunk.io-7f7fcc?label=docs&logo=readthedocs&labelColor=555555&logoColor=ffffff"/>
  </a>
    <a href="https://trunk.io">
    <img src="https://img.shields.io/badge/trunk.io-enabled-brightgreen?logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIGZpbGw9Im5vbmUiIHN0cm9rZT0iI0ZGRiIgc3Ryb2tlLXdpZHRoPSIxMSIgdmlld0JveD0iMCAwIDEwMSAxMDEiPjxwYXRoIGQ9Ik01MC41IDk1LjVhNDUgNDUgMCAxIDAtNDUtNDVtNDUtMzBhMzAgMzAgMCAwIDAtMzAgMzBtNDUgMGExNSAxNSAwIDAgMC0zMCAwIi8+PC9zdmc+"/>
  </a>
</p>

### Welcome

Toolbox is our custom collection of must have tools for any large software project. We've got a backlog of small tools to built into our toolbox here and happy to take contributions as well. `toolbox` is best used through `trunk check` to keep your development on rails (not the ruby kind).

This repo is open to contributions! See our
[contribution guidelines](CONTRIBUTING.md)

### Enabling

To enable the toolbox rules in your repository run:

```bash
trunk check enable trunk-toolbox
```

### Configuration

Toolbox can be configured via the toolbox.toml file. There is an example config file [here](.config/toolbox.toml). A full example file can be generated by calling

```bash
trunk-toolbox genconfig
```

### Rules

#### do-not-land

##### What it does

Keeps you from accidentally commiting code to a repository that is experimental, temporary, debugging cruft. It keeps you from pushing a PR with a bunch of printf() statements you added while debugging an error.

Valid triggers for this rule are: DONOTLAND, DO-NOT-LAND, DO_NOT_LAND, donotland, do-not-land, do_not_land

##### Why is this bad?

Anything you intentionally don't want in your repo should really not be there. This lets you flag the code you are writing to do testing without worrying that you'll forget you dropped it in your files before pushing your Pull Request.

##### Example

```typescript
// DONOTLAND
console.log("I don't think this code should execute");
```

#### TODO

##### What it does

Keeps you from accidentally commiting incomplete code to your repo. This is functionally the same as DONOTLAND, but reports at a lower severity so you can customize your approach to burning them down.

Valid triggers for this rule are: TODO, todo, FIXME, fixme

By default, this rule is disabled and must be enabled with:

```toml
[todo]
enabled = true
```

##### Why is this bad?

TODOs should be treated like any other lint issue. Sometimes you need to land code that still has these issues, but you want to keep track of them and avoid them when possible.

##### Example

```typescript
// TODO: We should evaluate using the asynchronous API
uploadResultsSync();
```

#### if-change-then-change

##### What it does

Allows you to enforce code synchronization. Often, we have code in one file that is reliant on code in another loosely - say an enum has 4 options and you want to make sure consumers of that enum are kept in sync as new enums are added. This rule will make sure the code is updated in both places when a modification occurs to the code block.

##### Why is this bad?

If code has baked-in assumptions that are not enforced through a check - then they can easily get out of sync. This rule allows you to encode that dependency and ensure all related code is updated when a modification occurs.

##### Example

This rule will report a violation if picker.rs is not updated when the content inside this enum block is modified:

```rust
let x = 7;

// IfChange
enum Flavor {
    Strawberry,
    Chocholate
}
// ThenChange srcs/robot/picker.rs

x += 9; // why not
```

#### never-edit

##### What it does

Allows you to enforce code does not get modified once checked into the repo.

##### Why is this bad?

If code is immutable - like database migration scripts - you want to ensure that no one edits those files
once they are checked in. This rule allows you to create restricted lists of files that cannot be edited
once added to the repo.

##### Example

This rule will report a violation if src/write_once.txt is modified or deleted in git given this config in toolbox.toml

```toml
[neveredit]
enabled = true
paths = ["**/write_once*"]
```

### Debugging Toolbox

Starting with release 0.5.0 toolbox now supports logging configuration using log4rs.yaml. toolbox will attempt to load this file from 
the currently executing directory. An example of this file is in the root directory of this repo. 

### Disclaimer

We know, we know...toolbox? That's only one step above 'UTILS', but a toolbox is a real thing, you can buy one in a store and put all kind of interesting things inside of them that make doing work a lot easier. Have you ever tried to change a wall socket without a screwdriver? We have...and it's not fun.
