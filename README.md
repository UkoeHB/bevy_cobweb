# Bevy Cobweb

Tool for building Bevy-integrated reactive webs.

- No macros
- No ambiguous jargon
- 100% Bevy
- Nodes are stateful reactive Bevy systems (stored on entities!)
- Robust change detection to avoid needlessly reinitializing and rerunning nodes
- Nodes react to resource mutations, entity changes, reactive events, and node-targeted events; recursive reactions allowed!
- Access node outputs throughout the web through node handles that synchronize with rebuilds
- Customizable error handling policy per root node with sane error propagation
- Detach and re-attach nodes anywhere in the web
- Unlimited root nodes


## Hello World

Here is an example for writing `"Hello, World!"` to the screen. Note that `bevy_cobweb_ui` does not exist yet.

```rust
use bevy::prelude::*;
use bevy_cobweb::{CobwebPlugin, NodeHandle, Web};
use bevy_cobweb_ui::{Location, ScreenArea, TextNode, WindowArea};

fn hello_world(mut web: Web<()>, window: Query<Entity, With<PrimaryWindow>>)
{
    let area: NodeHandle<ScreenArea> = WindowArea::new(window.single())
        .build(&mut web)
        .unwrap();

    TextNode::new()
        .text("Hello, World!")
        .location(area, Location::Relative((40., 60.), (40., 60.)))
        .build(&mut web)
        .unwrap();
}

fn main()
{
    App::new()
        .add_plugins(DefaultPlugins::default())
        .add_plugins(CobwebPlugin::default())
        .add_systems(Setup, hello_world)
        .run();
}
```



## `bevy` compatability

| `bevy` | `bevy_cobweb` |
|-------|----------------|
| 0.12  | 0.1 - master   |
