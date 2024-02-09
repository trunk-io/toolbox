<!-- trunk-ignore-all(trunk-toolbox) -->
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

### Enabling

To enable the toolbox rules in your repository run:

```bash
trunk check enable trunk-toolbox
```

### Rules

#### DONOTLAND

What it does
Keeps you from accidentally commiting code to a repository that is experimental, temporary, debugging cruft. Keeps your from pushing a PR with a bunch of printf() statements you added while debugging an error.

Why is this bad
Anything you intentionally don't want in your repo should really not be there. This lets you flag code you are writing to do testing without worrying that you'll forget you dropped it in your files before pushing your Pull Request.

Example
// DONOTLAND
console.log("I don't think this code should execute but if I see this statement in the logs...it has.);

#### IfChange (ThenChange)

What it does
Allows you to enforce code synchronization. Often we have code in one file that is reliant on code in another loosely - say an enum has 4 options and you want to make sure consumers of that enum are kept in sync as new enums are added. This rule will make sure code is updated in both places when a modication occurs to the code block.

Why is this bad
If code has baked in assumptions that are not enforced thru a check - then they can easily get out of sync. This rule allows you to encode that depedency and ensure all code points are updated when a modification occurs.

Example

```
// IfChange
enum Flavor {
Strawberry,
Chocholate
}
// ThenChange srcs/robot/picker.rs
```

// This rule will report a violation if picker.rs is not updated when the content inside this enum block is modified

### Disclaimer

We know, we know...toolbox? That's only one step above 'UTILS', but a toolbox is a real thing, you can buy one in a store and put all kind of interesting things inside of them that make doing work a lot easier. Have you ever tried to change a wall socket without a screwdriver? We have...and it's not fun.
