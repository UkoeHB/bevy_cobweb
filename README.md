# Bevy Cobweb (IN DEVELOPMENT)

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
- Automatic node cleanup with robust lifetime control



## Hello World

Here is a hypothetical example of writing `"Hello, World!"` to the screen. Note that `bevy_cobweb_ui` does not exist yet.

```rust
use bevy::prelude::*;
use bevy_cobweb::{CobwebPlugin, NodeHandle, SystemExt, Web};
use bevy_cobweb_ui::{Location, ScreenArea, TextNode, WindowArea};

fn hello_world(
    mut web : Web<()>,
    window  : Query<Entity, With<PrimaryWindow>>
) -> NodeResult<()>
{
    let area: NodeHandle<ScreenArea> = WindowArea::new(window.single())
        .build(&mut web)?;

    TextNode::new()
        .default_text("Hello, World!")
        .location(area, Location::Relative((40., 60.), (40., 60.)))
        .build(&mut web)?;

    Ok(())
}

fn main()
{
    App::new()
        .add_plugins(DefaultPlugins::default())
        .add_plugins(CobwebPlugin::default())
        .add_systems(Setup, hello_world.webroot())
        .run();
}
```

The `WindowArea` and `TextNode` types seen here are custom node builders that internally use the `bevy_cobweb` API. The `hello_world` system is a root node owned by a system produced by the [`.webroot()`](bevy_cobweb::webroot) system adaptor, which packages and stores the node in a `Local`. If the `hello_world` node errors-out, then the error handling policy configured in [`CobwebPlugin`](bevy_cobweb::CobwebPlugin) will be used (panic by default).

The `hello_world` node will rebuild if the window is resized, because `WindowArea` is internally set up to react to changes in the window size. When the `WindowArea`'s output changes, its parent node `hello_world` will be rebuilt automatically. When `hello_world` rebuilds, the `TextNode` child will also rebuild if `area` has changed, but its internal `Text` UI entity will be reused.



## Deep-Dive

This crate produces a structure analogous to a forest covered in cobwebs. Each 'tree' is a physical branching structure of nodes, and between all the nodes are reactive relationships (the 'web').


### Setup

The [`CobwebPlugin`](bevy_cobweb::CobwebPlugin) provides two configuration options:

- **[NodeErrorPolicy](bevy_cobweb::NodeErrorPolicy)**: The default error handling policy for root nodes. When an error propagates to a root node, the plugin's configured error handling policy will be used (e.g. panic, log and drop, etc.). Root nodes can override the plugin's default with a different policy (e.g. via `.webroot_with(NodeErrorPolicy::LogAndDrop)`).
- **[NodeCleanupPolicy](bevy_cobweb::NodeCleanupPolicy)**: The node cleanup policy for [`PackagedNodes`](bevy_cobweb::PackagedNode) (i.e. root nodes) that are dropped. All dropped [`PackagedNodes`](bevy_cobweb::PackagedNode) will be garbage collected, then cleaned up using the configured cleanup policy. Note that [`PackagedNodes`](bevy_cobweb::PackagedNode) can never be completely detached to live in the background; you must destroy them, attach them, or send them back to the garbage collector.


### Web Structure Overview

At the base of the web is a set of [`PackagedNodes`](bevy_cobweb::PackagedNode), each representing the root of a node tree. Packaged nodes are always root nodes, and any node in the tree can be packaged. Using this mechanic, you can detach any part of a node tree and reattach it anywhere else.


### Nodes

Every node is a stateful Bevy system that is operated by the web. Nodes have five pieces:

- **Node state**: This is mutable state tied to a specific node. Every time a node runs, it can use this state freely. Node state is initialized by the node builder, and can be overwritten with successive rebuilds (but the state type must stay the same).
- **Node input**: This is immutable data sent to a node by a node builder, and can be changed with successive rebuilds (but the input type must stay the same). Every time a node runs, it can read this data.
- **Node triggers**: These are ECS reaction triggers associated with the node. When a trigger is detected in the `World`, the node will automatically rebuild. Examples: resource mutation, entity component insertion, entity despawn, etc. The node builder specifies node triggers, and can change the node triggers with successive rebuilds (changing triggers does not force the node to run).
- **Node system**: This is the node's Bevy system. Running a node means running the node system on the Bevy `World`. The node builder specifies one node system for a given node (actually it can swap node systems that are only function pointers with the same signature, although not recommended).
- **Node output**: This is the node system's output. Node outputs come in two flavors: immutable data that can be read by anyone with a [`NodeHandle`](bevy_cobweb::NodeHandle) to that node, or consumable data in a [`NodeLink`](bevy_cobweb::NodeLink) that can be use to initialize a chained partner node.

Node systems are re-runnable 'constructors'. Every time a node runs, it needs to 'reconstruct' all of its child nodes. Children are tracked by `bevy_cobweb` and destroyed if not reconstructed when their parent runs. Node builders use change detection to avoid reconstructing a node if its state initialization or inputs have not changed, which minimizes the work needed to rebuild any node in the web.

#### Node State

Node state comes in two flavors.
- **Built-in**: Node state is moved into the node system, which is defined as a closure. This is easy to use, but node state can only be reset.
- **External**: Node state is defined outside the node system and you use a [`Node<S>`](bevy_cobweb::Node) system parameter to read the node state. This is less ergonomic, but enables node state merging (see the section below) and allows non-closure systems to be used.

#### Node Input

Node input data is readable through the [`Web<I>`](bevy_cobweb::Web) system parameter, which is also responsible for building child nodes.

#### Node Triggers

TODO

#### Inter-Node Dependencies via [NodeHandle\<O\>](bevy_cobweb::NodeHandle)

Nodes can have dependencies on each other through [`NodeHandles`](bevy_cobweb::NodeHandle).

TODO

- node handles not readable while node is building
    - nodes never readable by parents/ancestors, only non-direct relatives that are built *after* the handle is generated

#### Node Chaining via [NodeLink\<O\>](bevy_cobweb::NodeLink)

If you need node state initialization to be computed with a complex process, rather than embed that logic in the node that uses the state, you can use node chaining. The state-producer node will produce a [`NodeLink<O>`](bevy_cobweb::NodeLink), which is a consumable handle for the producer's output. The producer can then be 'connected' to another node by consuming the `NodeLink` in that node's node builder when setting or merging the node's state.

Producer nodes can only connect to sibling nodes with the same parent. Once a producer is connected to another node, it cannot be connected to any other node until the connected node is destroyed or detached.

If a producer is disconnected and has an empty `NodeLink` (i.e. because its previous connection consumed the link, and the link was not refreshed due to a rebuild of the producer triggered by something), it will be force-rebuilt the next time its link is used to connect another node. This makes it easy to seamlessly rearrange node chains.

Note that as with [`NodeHandles`](bevy_cobweb::NodeHandle), a connected node will not rebuild unless the `NodeLink` hash changes.

#### Node State Merging

Sometimes it's useful to incrementally update a node's state rather than completely reset it. In that case instead of setting node state in the node builder, you can merge existing state with initialization data (which may be data from the parent node, or a [`NodeLink`](bevy_cobweb::NodeLink)).

Nodes that allow state merging must use the [`Node<S>`](bevy_cobweb::Node) system parameter to access the node state.

#### Change Detection

TODO

- node building as deferred mutation, change detection as carefully ordered data hashing

- state initializers (normal, merged, normal/merged + NodeLink)
- node triggers
- node input
- node output

#### Built-in Nodes

TODO

**Reinitialization**

TODO

#### [PackagedNode](bevy_cobweb::PackagedNode)

A [`PackagedNode`](bevy_cobweb::PackagedNode) is a node in the web with no parent. Packaged nodes cannot be reinitialized, but they can be moved anywhere.

A `PackagedNode` can be scheduled to rebuild at any time (if you give it changed inputs), but does *not* produce a [`NodeHandle`](bevy_cobweb::NodeHandle), since handle access scoping is relative to a specific position within the web (whereas `PackagedNodes` have a 'floating' position). If you want to get a handle, then attach the node to another node to get a [`AttachedNode`](bevy_cobweb::AttachedNode) or [`ProducerNode`](bevy_cobweb::ProducerNode) before building it.

If a `PackagedNode` is dropped, then it will be sent to the `bevy_cobweb` garbage collector, where the preconfigured [`NodeCleanupPolicy`](bevy_cobweb::NodeCleanupPolicy) will decide what to do with it. Using a garbage collector makes it relatively safe to transfer `PackagedNodes` around your application (e.g. sending them between parts of the web through node events), since you won't be at risk of dangling nodes.

#### [AttachedNode](bevy_cobweb::AttachedNode)

A [`PackagedNode`] node that has been attached as a child of another node. It produces [`NodeHandles`](bevy_cobweb::NodeHandle).

An `AttachedNode` must be rebuilt every time its parent is rebuilt, otherwise it will be destroyed along with its children.

#### [ProducerNode](bevy_cobweb::ProducerNode)

A [`PackagedNode`] node that has been attached as a child of another node. It produces [`NodeLinks`](bevy_cobweb::NodeLink).

A `PackagedNode` must be rebuilt every time its parent is rebuilt, otherwise it will be destroyed along with its children.


### [NodeBuilder](bevy_cobweb::NodeBuilder)

All nodes are constructed using the [NodeBuilder](bevy_cobweb::NodeBuilder).

TODO


### Node Events

TODO


### ECS Reactivity

In `bevy_cobweb`, ECS reactivity is implemented through [`ReactCommands`](bevy_cobweb::ReactCommands). We use custom reactivity instead of Bevy change detection in order to achieve precise, responsive, recursive reactions with an ergonomic API that correctly integrates with `bevy_cobweb`'s node building protocol. In an ideal world `bevy_cobweb` would be upstreamed to Bevy, which would eliminate the ergonomic limitations of custom reactive elements (i.e. `ReactRes<>` resources and `React<>` components).

See the [docs](bevy_cobweb::react) for more details (WILL BE MIGRATED FROM `BEVY_KOT`, SEE [THE DOCS](https://github.com/UkoeHB/bevy_kot/tree/master/bevy_kot_ecs) THERE).

#### Three-Tier Commands Framework

A foundational component of `bevy_cobweb` is a three-tier commands framework that enables recursive rebuilds and reactions.

TODO

- bevy commands
- system commands
- reaction commands


### Error Propagation

TODO

- node errors vs accumulated web errors
- system command: ignore failing children (clear error queue)



## `bevy` compatability

| `bevy` | `bevy_cobweb` |
|-------|----------------|
| 0.12  | 0.1 - master   |
