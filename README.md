# Bevy Cobweb (IN DEVELOPMENT)

**There is no code yet, only the design draft in this document.**

Framework for building declarative reactive webs.

- Nodes are stateful reactive Bevy systems.
- Nodes react to resource mutations, entity changes, reactive events, and node events.
- Node outputs can be accessed throughout the web via node handles that synchronize with rebuilds.
- Change detection prevents reinitializing and rerunning nodes unless needed.
- Nodes may be detached and re-attached anywhere in the web.
- Nodes are automatically cleaned up when no longer used.
- Node error handling policy is customizable. Errors propagate to root nodes.
- Web mutations and node reactions are processed immediately and recursion is allowed.



## Motivation

This crate is intended as the foundation for building UI in Bevy, with the following goals in mind.

- No macros required, no third-party dependencies.
- Build UI declaratively in a Bevy-native style with heavy use of normal-looking Bevy systems.
- Minimize the separation between UI element construction and updating.
- Enable asset and editor-driven hot-reloading that preserves the existing UI state as much as possible.
- Provide a powerful, unopinionated API for building ergonomic UI widgets.
- Avoid unnecessary rebuilding as much as possible (efficiency is key).



## Hello World

Here is a hypothetical example of writing `"Hello, World!"` to the screen. Note that `bevy_cobweb_ui` does not exist yet.

```rust
use bevy::prelude::*;
use bevy_cobweb::{CobwebPlugin, NodeHandle, SystemExt, Web};
use bevy_cobweb_ui::{Location, ScreenArea, TextNode, TextSize, WindowArea};

fn hello_world(
    mut web : Web,
    window  : Query<Entity, With<PrimaryWindow>>
) -> NodeResult<()>
{
    let area: NodeHandle<ScreenArea> = WindowArea::new(window.single())
        .build(&mut web)?;

    TextNode::new()
        .default_text("Hello, World!")
        .location(area, Location::TotallyCentered)
        .size(area, TextSize::RelativeHeight(10.))
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

If the window is resized, then `WindowArea` will rebuild because it is internally set up to react to changes in the window size. When the `WindowArea`'s output changes, its parent node `hello_world` will be rebuilt automatically. When `hello_world` rebuilds, the `TextNode` child will also rebuild if `area` has changed, thereby propagating the window size change. Internally, `TextNode` will re-use its existing `Text` entity, avoiding string reallocation.



## Deep-Dive

If you are here for code, [skip ahead](#api-examples).

A web is a structure analogous to a forest covered in cobwebs. Each 'tree' is a physical branching structure of nodes, and between all the nodes are reactive relationships (the 'web').

There are two kinds of reactive relationships. One is ECS reactivity, where nodes will rebuild in response to changes in tracked ECS elements (resource mutations, entity changes, etc.). The other is inter-node dependencies, where nodes can depend on the outputs of upstream nodes. When a referenced node output changes, any nodes dependent on that output will rebuild.

Any node in the web can be detached from its parent and sent to be reattached elsewhere in the web. Root nodes can also be attached to other nodes.

As you might imagine, being able to reference the outputs of other nodes is both powerful and risky, especially given the ability to rearrange the web. We will discuss how to properly manage node references to ensure you avoid errors and bugs.


### Web Structure Overview

At the base of the web are [`RootNodes`](bevy_cobweb::RootNode), each representing the root of a node tree. Root nodes are simple wrappers around packaged [`BasicNodes`](bevy_cobweb::BasicNode), which means they can be created from normal nodes in the web. Root nodes are not allowed to depend on other nodes, and they do not have outputs (specifically, they may only wrap a `BasicNode<S, I, ()>`, where `S` and `I` implement `Hash`).

Every node in the tree can have child nodes. Child nodes come in two types, built-in nodes that are tracked implicitly, and object nodes that can be packaged and relocated ([`BasicNodes`](bevy_cobweb::BasicNode), [`ProducerNodes`](bevy_cobweb::BasicNode), and [`ReactorNodes`](bevy_cobweb::BasicNode)). Parents track their child nodes through 'node names', which are unique identifiers that allow the web to compare node metadata between successive rebuilds. Built-in child nodes can either be explicitly named by the user, or are assigned an anonymous name based on their index in the list of anonymous built-in child nodes. Object node names are derived from their unique node ids, which are global ids within the web.

Building a node involves specifying the node's initial state, input, reaction triggers, and internal system. Exactly how these are specified depends on the node type, which we will discuss in later sections. The first time a node is built, its internal system will be scheduled to run. After a node system runs, its output will be saved in the web for downstream nodes to read. We discuss the `bevy_cobweb` scheduling algorithm in [Scheduling](#scheduling).

When a node runs its node system, it will detect which of its children were built, and then destroy any children that had been built in the past but failed to rebuild. It does this by comparing the child node name lists before and after the rebuild. Anonymous built-in, named built-in, and object nodes will all be destroyed if not rebuilt. Object child nodes cannot be rebuilt after being destroyed, but built-in node names can be reused for new nodes.

Building a child node uses change detection to avoid re-running the child node's internal node system unless necessary. The node state and node input passed to a node when building it are compared against hashes of state and input used the last time the node was built. If the hashes are the same, then the node system will not be scheduled and its output will stay the same.

As mentioned, child nodes can be assigned reaction triggers, which specify which ECS mutations will cause the node system to re-run automatically. When a node runs after being triggered, it will re-use its existing state and input, and will produce a new output. Node state is mutable and node inputs are immutable. Note that assigning a child node new reaction triggers will not cause the node to re-run, but it *will* cause pending reactions targeting that node to fail.

Child node outputs are *deferred*, which means they cannot be accessed by their parents. However, they *can* be safely accessed by downstream sibling nodes, and child node output handles can be returned by a parent node for use in cousin nodes (but not direct ancestors). In order to perform change detection on data (node state/inputs/outputs) that may contain handles to the deferred outputs of other nodes, the node scheduler carefully orders events so that node outputs will be fully resolved by the time they might be needed for performing change detection in dependents. We compute change hashes using a custom [`NodeHash`](bevy_cobweb::NodeHash) trait that allows inspecting the contents of handles (which are not themselves hashable). This is the 'memoization magic' of `bevy_cobweb`.

A triggered node can cause its parent to rebuild in two scenarios. One is if the node's output changes. The other is if the node errors out. We rebuild parents on error because it's possible that an error was caused by a failure to read an invalid node handle, and so we give the parent an opportunity to repair its children (parents also have the chance to discard errors before they can propagate further). Parents will recursively rebuild until the algorithm reaches an ancestor whose output doesn't change and that doesn't error out, or until the root node is rebuilt. Root nodes don't have outputs, and they consume propagated errors using their configured error handling policy.

A triggered node can error out either because it directly returned a [`NodeError`](bevy_cobweb::NodeError), or because a node error was propagated up by one of its children. Propagated errors are collected internally in [`WebErrors`](bevy_cobweb::WebError), which are only readable if they propagate to a root node and are consumed by its error handling policy.

Last but not least, child object nodes can be detached from their parents and reattached elsewhere in the web. This is done with [`Packaged`](bevy_cobweb::Packaged) node wrappers that can be sent through node events directly to other nodes (where they can be reattached and rebuilt). A node event is a special feature of `bevy_cobweb` that facilitates web mutations. One significant risk of moving a branch across the web is that if a node in the in-transit branch is triggered by an ECS mutation, then node references in the triggered node may be invalid and the reaction will error-out. To address this, the `bevy_cobweb` scheduler ensures that node events are fully resolved before processing ECS reactions. This allows branches transmitted by node events to safely reattach and then repair any internal node references by rebuilding, before they can be accessed erroneously. Of course, users can always shoot themselves in the foot by improperly handling node references, which is a weakness of this design.


### Plugin

The [`CobwebPlugin`](bevy_cobweb::CobwebPlugin) is the starting point for using `bevy_cobweb`. It provides two configuration options.

- **[NodeErrorPolicy](bevy_cobweb::NodeErrorPolicy)**: The default error handling policy for root nodes (e.g. panic, log and drop, etc.). Root nodes can override the plugin's policy with a different policy (e.g. via `.webroot_with(NodeErrorPolicy::LogAndDrop)`). When an error propagates to a root node, its configured error handling policy will consume the error.
- **[NodeCleanupPolicy](bevy_cobweb::NodeCleanupPolicy)**: The pre-configured node cleanup policy for [`RootNodes`](bevy_cobweb::RootNode) and [`Packaged`](bevy_cobweb::Packaged) nodes that are dropped. When a root or packaged node is dropped, it will be garbage collected, then cleaned up using the configured cleanup policy. Note that root and packaged nodes can never be completely detached to live in the background. You must store them, destroy them, attach them to other nodes, or send them back to the garbage collector.


### [Web](bevy_cobweb::Web)

The [`Web`](bevy_cobweb::Web) is a system parameter used to build nodes, attach and detach nodes, read node outputs, and send node events.


### Nodes

Every node is a stateful Bevy system that is operated by the web. Nodes have five pieces:

- **Node state**: This is mutable state tied to a specific node. Every time a node runs, it can use this state freely. Node state is initialized by the node builder, and can be overwritten or updated with successive rebuilds. Node state comes in two flavors:
    - **Built-in**: Node state is moved into the node system, which is defined as a closure. This is easy to use, but node state can only be reset.
    - **External**: Node state is defined outside the node system and can be accessed via the [`NodeState<S>`](bevy_cobweb::NodeState) system parameter (which panics on deref if the state is missing). This is less ergonomic, but enables node state merging (see [Node State Merging](#node-state-merging)) and allows non-closure systems to be used.
- **Node input**: This is immutable data sent to a node by a node builder, and can be changed with successive rebuilds. Every time a node runs, it can read this data via the [`NodeInput<I>`](bevy_cobweb::NodeInput) system parameter (which panics on deref if the input is missing).
- **Node triggers**: These are ECS reaction triggers associated with a node. When a trigger is detected in the `World`, the node will automatically rebuild. Examples: resource mutation, entity component insertion, entity despawn, etc. The node builder specifies node triggers, and can change the node triggers with successive rebuilds (changing triggers does not force the node to run, but it does invalidate all pending reactions targeting that node). See [ECS Reactivity](#ecs-reactivity).
- **Node system**: This is the node's Bevy system. Running a node means running the node system on the Bevy `World`.
- **Node output**: This is the node system's output. Node outputs come in two flavors:
    - **Read-only**: Immutable data that can be read by anyone with a [`NodeHandle`](bevy_cobweb::NodeHandle) to that node. Node handles are produced by built-in basic nodes and object-type [`BasicNodes`](bevy_cobweb::BasicNode).
    - **Consumable**: Single-use data in a [`NodeLink`](bevy_cobweb::NodeLink) that can be use to initialize a chained partner node (produced by producer nodes). Node links are produced by built-in producer nodes and object-type [`ProducerNodes`](bevy_cobweb::ProducerNode).

Node systems are re-runnable 'constructors'. Every time a node runs, it needs to 'reconstruct' all of its child nodes. Children are tracked by `bevy_cobweb` and destroyed if not reconstructed when their parent runs. Node builders use change detection to avoid reconstructing a node if its state initialization or inputs have not changed, which minimizes the work needed to rebuild any node in the web.

#### Inter-Node Dependencies via [NodeHandle\<O\>](bevy_cobweb::NodeHandle)

Node handles represent a reference to a specific node's output that is stored in the web. They also contain a [`NodeId`](bevy_cobweb::NodeId) that can be used to send node events to the referenced node.

The data in a node handle is not readable while the node is building. This means node handles are only readable within sibling and cousin nodes that are built downstream of the handles' origins. The are *not* readable by parents and direct ancestors, which always finish running before their children.

It is best to think of node handles as `&O` Rust references that need to be manually managed. They are backward-facing references, which means they should (in most cases) only be sent forward in a node tree, and should not be passed between node trees or outside of the web. They should also not be stored in places that won't be refreshed if the node handle is no longer valid (unless you can guarantee they won't be erroneuosly accessed).

Here are guidelines for safe usage of node handles:
- Only pass node handles via node state/input/output.
- Never store handles in node state that won't be updated if the handle becomes invalid (e.g. it is not recommended to push handles into a container when merging new state, and then never clean up existing handles).
- Never store handles in `Resources` or `Components`.
- Never store [`Packaged`](bevy_cobweb::Packaged) nodes anywhere if they contain any reactive nodes or may be the target of node events.
    - Only transmit [`Packaged`](bevy_cobweb::Packaged) nodes with node events, to ensure they are reattached before any reactions can be executed.
- Keep in mind that node events that target nodes in [`Packaged`](bevy_cobweb::Packaged) nodes can error out if the [`Packaged`](bevy_cobweb::Packaged) contains stale handles. This kind of error will only occur if a node event is sent before a [`Packaged`](bevy_cobweb::Packaged) node event is sent, since the [`Packaged`](bevy_cobweb::Packaged) node event should reinsert and repair the `Packaged`'s internal nodes in the reverse case.
    - Recommendation: Node events should only point to nodes behind them in the web, not in front of them (this matches the expected use of node handles, which are backward-pointing references).

If you access node handles with ambiguous lineage, then reading a handle can fail unexpectedly and randomly. You can manually validate handle lineage, but be warned that doing so is fragile and laborious.

Probable sources of error are:
- A node is moved, stored handles are not repaired, and stored handles point to other trees or forward in the current tree.
- A handle is transmitted somewhere without using the `bevy_cobweb` API.

#### Node Chaining via [NodeLink\<O\>](bevy_cobweb::NodeLink)

If you need node state initialization to be computed with a complex process, rather than embed that logic in the node that uses the state, you can use node chaining. The producer node will output a [`NodeLink<O>`](bevy_cobweb::NodeLink), which is a consumable handle. Another node can then 'connect' to the producer by consuming the `NodeLink` when updating their node state.

Producer nodes (both built-in producer nodes and [`ProducerNode`](bevy_cobweb::ProducerNode) object nodes) can only connect to sibling nodes with the same parent. Once a producer is connected to another node, it cannot be connected to any other node until the connected node is destroyed or detached.

A producer only sets a new value to its `NodeLink` when rebuilt. If a producer is not connected to a consumer and has an empty `NodeLink` (i.e. because its previous connection consumed the link, and the link was not refreshed by a rebuild), then the next time its link handle is used to connect another node it will be force-rebuilt. This makes it easy to seamlessly rearrange node chains.

A consumer node can only consume one `NodeLink` and no other values when updating their node state. Unlike other aspects of the web design, `NodeLinks` do not use change detection because they can transmit non-hashable data (e.g. [`Packaged`](bevy_cobweb::Packaged) nodes). Whenever a non-empty node link is consumed, the consumer will always rebuild.

#### Node State Merging

Often it's useful to incrementally update a node's state rather than completely reset it. In that case instead of setting node state in the node builder, you can merge existing state with initialization data (which may be data from the parent node, or a [`NodeLink`](bevy_cobweb::NodeLink)).

Nodes that allow state merging must use the [`NodeState<S>`](bevy_cobweb::Node) system parameter to access the node state.

#### [NodeHash](bevy_cobweb::NodeHash) Change Detection

In `bevy_cobweb`, building a node is a 'deferred web mutation'. Building a node schedules a system command (see [Scheduling](#scheduling)), and the node output is not available until that command has been executed. However, we allow nodes to depend on the outputs of other nodes *and* we perform change detection so that nodes won't rebuild unless their inputs change (or their children's outputs change, in the case of children reacting to an ECS mutation).

This presents an ordering issue. If a child node depends on the output of an older sibling node, how do we perform change detection when deciding if the child node should be built? This is tricky because the sibling node's output may internally contain references to its own children.

Our solution is to schedule 'node building' and 'node output handling' as two separate system commands. Since system commands are telescoped (see [Scheduling](#scheduling)), all the children of the 'node building' step will be built before the 'node output handling' step is completed. This way all internal dependencies of a node output can be resolved before inspecting that output.

Since node references ([`NodeHandles`](bevy_cobweb::NodeHandle)) are not directly hashable, we use a custom trait [`NodeHash`](bevy_cobweb::NodeHash) that allows inspecting the contents of node references in order to hash them. This trait is automatically implemented for types that implement `Hash`.

Change detection is performed in the following areas:
- **Node state data**: Including captured data, external data, merged external data, and [`NodeLinks`](bevy_cobweb::NodeLink). Node links are not actually hashed since they may contain non-hashable data (primarly [`Packaged`](bevy_cobweb::Packaged) nodes) (and node links are consumable so we always want to rebuild when link data is provided), so we hash a 'link counter' instead which tracks how many times a non-empty link has been consumed (as a proxy for change detection).
- **Node triggers**: Including directly-supplied triggers and derived triggers. These just use `Hash` since triggers cannot contain deferred data.
- **Node inputs**: These are straightforward.
- **Node outputs**: These are straightforward, however if a node output is an error then computing a [`NodeHash`](bevy_cobweb::NodeHash) for its [`NodeHandle`](bevy_cobweb::NodeHandle) will also error. Errors are contagious in this sense.


### [NodeBuilder](bevy_cobweb::NodeBuilder)

All nodes are constructed using the [`NodeBuilder`](bevy_cobweb::NodeBuilder) helper type.

Building a node has five steps:

1. Initialize the builder. This is where the node state and node system are specified.
    - **Simple**: `new(system)`. The node is state-less.
    - **Built-in state**: `new_with(state, system)`. The node state is reset when the initial state changes, and is moved into the system which must be a closure.
    - **External state**: `from(state, system)`. The node state is reset when the initial state changes, and is accessed with [`NodeState<S>`](bevy_cobweb::NodeState).
    - **External state merged**: `from_merged(init data, merge callback, system)`. The existing node state is merged with initial state data when that data changes, and is accessed with [`NodeState<S>`](bevy_cobweb::NodeState).
    - **NodeLink**: `connect(node link, system)`. The node becomes connected to the [`NodeLink`](bevy_cobweb) source. The node state is reset when a non-empty link is received.
    - **NodeLink merged**: `connect_merged(node link, merge callback, system)`. The node becomes connected to the [`NodeLink`](bevy_cobweb) source, and existing node state is merged with the node link contents if the link is non-empty.
1. Specify reaction triggers (optional).
    - **Direct triggers**: `triggers(trigger bundle)`. Triggers are reset when the trigger bundle changes.
    - **Deferred triggers**: `triggers_from(callback)`. Triggers are derived from a callback that takes `&Web`, allowing them to be derived from the node handles of older siblings.
1. Specify node input (optional).
    - **Input**: `input(input)`. The node input is reset when the input value changes, and is accessed with [`NodeInput<I>`](bevy_cobweb::NodeInput).
1. Finalize the builder. This is where you make either a built-in node or an object node.
    - **Built-in node**: Nodes are built into the current parent. Nodes can be given a name manually, otherwise they are assigned a name based on their index in the parent's anonymous node list. Manually naming is useful if your built-in nodes may be constructed with different orderings or conditionally constructed. These cannot be created if there is no parent node.
        - **Basic node**: `build{_named}(&mut web{, name})`. Outputs a [`NodeHandle<O>`](bevy_cobweb::NodeHandle).
        - **Producer node**: `build_producer{_named}(&mut web{, name})`. Outputs a [`NodeLink<O>`](bevy_cobweb::NodeLink).
        - **Reactor node**: `build_reactor{_named}(&mut web{, name})`. Outputs a [`NodeId`](bevy_cobweb::NodeId). This does not actually run the node system, it just prepares the node and registers it as a built-in child.
    - **Object node**: Nodes are prepared as children but not built. If you don't build or package a basic or producer object node then it will be destroyed.
        - **BasicNode**: `prepare(&mut web)`. Outputs a [`BasicNode<S, I, O>`](bevy_cobweb::BasicNode). The basic node can be further converted into a [`RootNode<S, I>`](bevy_cobweb::RootNode) if `S` and `I` implement `Hash` and `O` is `()`.
            - **RootNode**: `as_root()`.
            - **RootNode with error policy**: `as_root_with(policy)`. Consumes a custom [`NodeErrorPolicy`](bevy_cobweb::NodeErrorPolicy).
        - **ProducerNode**: `prepare_producer(&mut web)`. Outputs a [`ProducerNode<S, I, O>`](bevy_cobweb::ProducerNode).
        - **ReactorNode**: `prepare_reactor(&mut web)`. Outputs a [`ReactorNode<S>`](bevy_cobweb::ReactorNode). You can't make one of these unless `I` and `O` are `()`.
1. Object nodes can then be built into the current parent:
    - **Build BasicNode**: `build(&mut web)`. Outputs a [`NodeHandle<O>`](bevy_cobweb::NodeHandle).
    - **Build ProducerNode**: `build(&mut web)`. Outputs a [`NodeLink<O>`](bevy_cobweb::NodeLink).
    - **Build ReactorNode**: `build(&mut web)`. Outputs a [`NodeId`](bevy_cobweb::NodeId).

#### [BasicNode](bevy_cobweb::BasicNode)

Once you have a [`BasicNode`](bevy_cobweb::BasicNode) object, you can specify new state or input before building it, or override existing triggers.

```rust
basic_node
    .state(abc)
    .triggers(resource_mutation::<A>)
    .input(xyz)
    .build(&mut web)?;
```

The same pattern works whether the internal node resets its state, merges its state, or consumes a [`NodeLink`](bevy_cobweb::NodeLink).

#### [ProducerNode](bevy_cobweb::ProducerNode)

As with `BasicNode`, when you have a [`ProducerNode`](bevy_cobweb::ProducerNode) object, you can specify new state or input before building it, or override existing triggers.

```rust
basic_node
    .state(abc)
    .triggers(resource_mutation::<A>)
    .input(xyz)
    .build(&mut web)?;
```

Again, the same pattern works whether the internal node resets its state, merges its state, or consumes a [`NodeLink`](bevy_cobweb::NodeLink). Note that nodes are free to both consume and output node links, for producer-consumer chains of arbitrary length.

Keep in mind that once a node has consumed a producer's `NodeLink`, no other node can consume it until the original consumer has been destroyed or detached.

#### [ReactorNode](bevy_cobweb::ReactorNode)

[`ReactorNodes`](bevy_cobweb::ReactorNode) are somewhat simpler, you can specify new state or override existing triggers. Building one of these nodes doesn't actually run the internal system, but it *is* required to call `build()` so the web can track it.

```rust
basic_node
    .state(abc)
    .triggers(resource_mutation::<A>)
    .build(&mut web)?;
```

And again, the same pattern works whether the internal node resets its state, merges its state, or consumes a [`NodeLink`](bevy_cobweb::NodeLink).

#### [Packaged](bevy_cobweb::Packaged) Nodes

Node objects can be detached from their parents by packaging them into [`Packaged`](bevy_cobweb::Packaged) nodes. Packaged nodes can be moved around freely, and reattached anywhere in the web.

A packaged node cannot be directly built, but it may run anyway if triggered by a reaction. Since a packaged node is prone to having invalid node references, we avoid processing reactions until all pending web mutations have been resolved. This gives users a chance to reattach and rebuild packaged nodes, and hopefully repair any disjoint node handles in the detached branch. We recommend moving packaged nodes around with node events, since those are always handled before ECS reactions.

A packaged node may also run if it was built before being packaged, although that is less likely to error-out. In the event that an error does occur within a packaged node, the error (once it propagates to the packaged node) will be discarded. When the node is reattached and rebuilt, if the error reoccurs then it will propagate to the appropriate root node and be handled there.

If a `Packaged` node is dropped, then it will be sent to the `bevy_cobweb` garbage collector, where the pre-configured [`NodeCleanupPolicy`](bevy_cobweb::NodeCleanupPolicy) will decide what to do with it. Using a garbage collector makes it relatively safe to transfer `Packaged` nodes around your application (e.g. sending them between parts of the web through node events), since you will always have a chance to recover from problems (i.e. nodes will never dangle).

#### [RootNode](bevy_cobweb::RootNode)

A `RootNode` can be produced from a `Packaged<BasicNode<S, I, ()>>` if `S` and `I` implement `Hash`. We don't allow root nodes to depend on other nodes in order to support safe use of node handles, which should be carefully controlled.

#### Reinitializing Nodes

When a parent node is reinitialized (i.e. its node state is overwritten), it is important to keep in mind what will happen to the node's children.
- Built-in children will reinitialize if new initialization state is given in their node builder, otherwise they will be preserved. Any built-in child names that aren't rebuilt will be destroyed by the parent after it is done building (as normal).
- [`RootNodes`](bevy_cobweb::RootNode) and [`Packaged`](bevy_cobweb::RootNode) nodes stored in the old node state will be dropped and garbage collected. If you want to retain these nodes, then the parent node should merge its state rather than resetting it.
- [`BasicNodes`](bevy_cobweb::BasicNode), [`BasicNodes`](bevy_cobweb::BasicNode), and [`BasicNodes`](bevy_cobweb::BasicNode) stored in the old node state will be dropped and then completely destroyed by the parent after it is done rebuilding (after reinitializing). If you want to retain these nodes, then the parent node should merge its state rather than resetting it.

Note that if your system contains a `Local`, then it will always be reset if node state is captured by a closure. Otherwise it will be preserved across node state resets.


### Error Propagation

All node systems must return `Result<O, NodeError>`. Node errors are collected by `bevy_cobweb`'s internal `WebCache` and saved in a [`WebError`](bevy_cobweb::WebError), which contains a trace of all node errors. If a node experiences an error, the node error will be saved but all sibling nodes will still run (and their errors will also be saved).

A parent node's output will internally be treated as an error if any of its children have errored-out. This will cause errors to propagate upward until they hit a [`Packaged`](bevy_cobweb::Packaged) node or a [`RootNode`](bevy_cobweb::RootNode). Errors that hit a packaged node are dropped, while errors that hit a root node are consumed by the node's configured [`NodeErrorPolicy`](bevy_cobweb::NodeErrorPolicy).

Since error propagation isn't always desired, the [`Web`](bevy_cobweb::Web) system parameter includes two error helpers:
- `.ignore_failing_children()`: This inserts a system command that clears errors detected for children of the current parent node. Note that errors from children built after this is invoked won't be ignored, and errors from the parent won't be ignored.
- `.capture_errors()`: This inserts a system command with a user-defined callback for consuming child errors.


### Node Events

Node events allow you to send one-off data directly to a node system. Given a [`NodeId`](bevy_cobweb::NodeId), you simply call `web.send(node_id, event_data)` and then in the target node can use the [`NodeEvent<E>`](bevy_cobweb::NodeEvent) system parameter to take the data.

Under the hood, node events use [`SystemEvents`](bevy_cobweb::SystemEvent), which use a system id to send event data.

We discuss the implementation of system events further in [Scheduling](#scheduling).


### ECS Reactivity

In `bevy_cobweb`, ECS reactivity is implemented through [`ReactCommands`](bevy_cobweb::ReactCommands). We use custom reactivity instead of Bevy change detection in order to achieve precise, responsive, recursive reactions with an ergonomic API that correctly integrates with `bevy_cobweb`'s node building protocol. In an ideal world `bevy_cobweb` would be upstreamed to Bevy, which would eliminate the ergonomic limitations of custom reactive elements (i.e. `ReactRes<>` resources and `React<>` components).

Note that 'observers' are currently planned for Bevy-native reactivity, however it is not clear that the [proposed implementation](https://github.com/bevyengine/bevy/pull/10839) is compatible with `bevy_cobweb`'s API and scheduling invariants.

See the [docs](bevy_cobweb::react) for more details (TODO: will be migrated from `bevy_kot` then updated, see [the docs](https://github.com/UkoeHB/bevy_kot/tree/master/bevy_kot_ecs) there).


### Scheduling

A foundational component of `bevy_cobweb` is a four-tier commands framework (aka the scheduler) that enables recursive rebuilds and reactions.

Conceptually, the four tiers are as follows:

1. Inner-system commands: single-system ECS mutations and system-specific deferred logic.
1. System commands: propagation of a single system-web mutation (a rebuild of one node). Building a child node will schedule system commands for running the node system and handling the node output.
1. System events: propagation of a system-web transaction, which may be composed of multiple system-web mutations (e.g. reinserting a branch is a transaction).
1. Reactions: queued system-web transactions triggered by ECS mutations. We consider normal reactions as 'single-node webs'.

Each tier expands in a telescoping fashion. When one `Command` is done running, all commands queued by that `Command` are immediately executed before any previous commands, and so on for the other tiers.

There are two important innovations that the `bevy_cobweb` command-resolver algorithm introduces.
- **Rearrange `apply_deferred`**: Every Bevy system can have internally deferred logic that is saved in system parameters. After a system runs, that deferred logic can be applied by calling `system.apply_deferred(&mut world)`. The problem with this is if the deferred logic includes triggers to run the same system again (e.g. because of reactivity), an error will occur because the system is currently in use. To solve this, `bevy_cobweb` only uses `apply_deferred` to apply the first command tier. Everything else is executed after the system has been returned to the world.
- **Injected cleanup**: In `bevy_cobweb` you access node state, node input, and node events with the [`NodeState`](bevy_cobweb::NodeState), [`NodeInput`](bevy_cobweb::NodeInput) and [`NodeEvent`](bevy_cobweb::NodeEvent) system parameters. In order to properly set the underlying data of these parameters such that future system calls won't accidentally have access to that data, our strategy is to insert the data to custom resources immediately before running node systems and then remove that data immediately after the system has run but before calling `apply_deferred`. We do this with an injected cleanup callback in the system runner.

In order to rearrange `apply_deferred` as described, all system commands, system events, and reactions are queued within a central `ReactCache` resource.

The scheduler has three pieces. Note that all systems in this context are custom one-shot systems stored on entities.

**1. System runner**

At the lowest level is the system runner, which executes a single scheduled system. All Bevy `Commands` and system commands created by the system that is run will be resolved here.

1. Remove the target system from the `World`.
  - If the system is missing, run the cleanup callback and return.
1. Remove pre-existing pending system commands. While not useful directly within `bevy_cobweb`, this allows reactive systems to be called manually within other reactive systems.
1. Run the system on the world: `output = system.run(input, world)`.
1. Invoke the cleanup callback.
1. Apply deferred: `system.apply_deferred(world)`.
1. Reinsert the system into the `World`.
1. Take pending system commands from the `ReactCache` and run them recursively with this system runner.
    - After running one system command, take pending system commands again and push them to the front of the queue. Continue until all system commands are done.
1. Replace pre-existing pending system commands that were removed.

**2. System events**

When [`ReactCommands`](bevy_cobweb::ReactCommands) receives a system-targeted event with `rc.system_event(sys_id, event)`, it wraps the system invocation and event data in a closure and inserts it into the system event queue.

That closure does the following:

1. Insert event data into system event resource.
1. Call the system runner for the target system.
    - Use a cleanup callback that clears the system event resource.

Raw system events are readable with the [`SystemEvent`](bevy_cobweb::SystemEvent) system parameter, but we wrap that in [`NodeEvent`](bevy_cobweb::NodeEvent) in the web for a slightly more ergonomic API tailored to the web use-case (`SystemEvents` created for node events include the node id that originated the node event, and `NodeEvent` makes it easier to read the node event data).

**3. Command resolver**

When a system command, system event, or reaction is scheduled in 'user-land' (aka a normal Bevy system outside `bevy_cobweb`), we create a normal Bevy `Command` that launches a reaction tree. This reaction-resolver command is *only* emitted when we are not already inside a reaction tree.

The reaction-resolver will fully execute all recursive system commands, system events, and reactions before returning to user-land. The resolver command's algorithm is as follows:

1. Set the reaction tree flag to prevent the resolver from being recursively scheduled.
1. Set up three queues: system commands, system events, reactions.
1. Loop until there are no pending system commands, system events, or reactions.
    1. Loop until there are no pending system commands or system events.
        1. Loop until there are no pending system commands.
            1. Remove pending system commands and push them to the front of the system commands queue.
            1. Pop one system command from the queue and run it with the system runner.
        1. Remove pending system events and push them to the front of the system events queue.
        1. Pop one system event from the queue and run it.
    1. Remove pending reactions and push them to the front of the reactions queue.
    1. Pop one reaction from the queue and run it.
1. Unset the reaction tree flag now that we are returning to user-land.



## API Examples

Here are a bunch of examples of using `bevy_cobweb`'s raw API. In practice, users of this crate would use a mixture of the raw API and ergonomic wrappers such as those showcased in the [Hello World](#hello-world).

For more speculative examples of how `bevy_cobweb` might be used in a real UI, [jump ahead](#speculative-examples).


### Making a [RootNode](bevy_cobweb::RootNode)

Making a root node is as simple as packaging a [`BasicNode`](bevy_cobweb::BasicNode) and then converting it with [`.as_root()`](bevy_cobweb::BasicNode::as_root). Keep in mind that root nodes need to be stored somewhere otherwise they will be garbage collected.

Here is the implementation for the [`.webroot()`](bevy_cobweb::webroot) system extension:

```rust
fn webroot<M>(
    node: impl IntoSystem<(), (), M> + 'static
) -> impl FnMut<(Local<Option<RootNode<(), ()>>>, Web)> + 'static
{
    move |mut cached: Local<Option<RootNode<(), ()>>>, mut web: Web|
    {
        if cached.is_some() { return; }
        let root = NodeBuilder::new(node)
            .prepare(&mut web).unwrap()
            .packaged(&mut web).unwrap()
            .as_root();
        root.build(&mut web).unwrap();
        *cached = Some(root);
    }
}
```

You can also configure the error handling policy for root nodes. Here is the implementation for the [`.webroot_with()`](bevy_cobweb::webroot_with) system extension:

```rust
fn webroot_with<M>(
    node   : impl IntoSystem<(), (), M> + 'static,
    policy : impl Into<NodeErrorPolicy> + 'static,
) -> impl FnMut<(Local<Option<RootNode<(), ()>>>, Web)> + 'static
{
    move |mut cached: Local<Option<RootNode<(), ()>>>, mut web: Web|
    {
        if cached.is_some() { return; }
        let root = NodeBuilder::new(node)
            .prepare(&mut web).unwrap()
            .packaged(&mut web).unwrap()
            .as_root_with(policy.into());
        root.build(&mut web).unwrap();
        *cached = Some(root);
    }
}
```


### Node State Examples

A node can be built with no state with [`.new()`](bevy_cobweb::NodeBuilder::new)

```rust
fn no_state(mut web: Web) -> NodeResult<()>
{
    NodeBuilder::new(
            || -> NodeResult<()>
            {
                println!("empty node");
                Ok(())
            }
        )
        .build(&mut web)?;
    Ok(())
}
```

Or with captured state with [`.new_with()`](bevy_cobweb::NodeBuilder::new_with). Note that the captured state is moved into the system via an intermediary closure. It is a compile error to capture anything from the environment.

```rust
fn captured_state(mut web: Web) -> NodeResult<()>
{
    let c = 0;
    NodeBuilder::new_with(
            c,
            |mut c| move || -> NodeResult<()>
            {
                c += 1;
                println!("we ran {c} times");
                Ok(())
            }
        )
        .build(&mut web)?;

    Ok(())
}
```

Or by storing the state separately with [`.from()`](bevy_cobweb::NodeBuilder::from) and accessing it with [`NodeState<S>`](bevy_cobweb::NodeState). This is necessary if your node system is a function pointer.

```rust
fn my_node(mut state: NodeState<usize>) -> NodeResult<()>
{
    *state += 25;
    println!("{state}");
    Ok(())
}

fn from_state(mut web: Web) -> NodeResult<()>
{
    NodeBuilder::from(0, my_node).build(&mut web)?;
    Ok(())
}
```

Or by merging existing state with new state with [`.from_merged()`](bevy_cobweb::NodeBuilder::from_merged). We need [`NodeState<S>`](bevy_cobweb::NodeState) to access the node state, which is stored separate from the node system in order to merge it with updates.

```rust
fn from_state_merged(mut web: Web) -> NodeResult<()>
{
    let c = 100;
    NodeBuilder::from_merged(
            c,
            |old: Option<usize>, new: usize| -> MergeResult<usize>
            {
                Ok(old.map_or_else(
                    || new,
                    |old| new + *old
                ))
            }, 
            |mut state: NodeState<usize>| -> NodeResult<()>
            {
                *state *= 2;
                println!("{state}");
                Ok(())
            }
        )
        .build(&mut web)?;

    Ok(())
}
```


### Node Input Examples

Data passed as an input with [`.input()`](bevy_cobweb::NodeBuilder::input) is readable with [`NodeInput<I>`](bevy_cobweb::NodeInput).

```rust
fn input(mut web: Web) -> NodeResult<()>
{
    NodeBuilder::new(
            |input: NodeInput<usize>| -> NodeResult<()>
            {
                println!("{:?}", *input);
                Ok(())
            }
        )
        .input(10)
        .build(&mut web)?;

    Ok(())
}
```


### Node Triggers Examples

A node can react to ECS triggers with [`.triggers()`](bevy_cobweb::NodeBuilder::triggers).

```rust
fn parent_of_sensitive_child(mut web: Web) -> NodeResult<()>
{
    NodeBuilder::new(
            || -> NodeResult<()>
            {
                println!("Stop triggering mee!!!");
                Ok(())
            }
        )
        .triggers(resource_mutation::<UnorganizedCode>())
        .build(&mut web)?;

    Ok(())
}

fn unorganize_it(mut rc: ReactCommands, mut uc: ReactResMut<UnorganizedCode>)
{
    uc.get_mut(&mut rc).jumble();
}

fn main()
{
    App::new()
        .add_plugins(DefaultPlugins::default())
        .add_plugins(CobwebPlugin::default())
        .add_systems(Startup, parent_of_sensitive_child.webroot())
        .add_systems(Update, unorganize_it)
        .init_react_resource::<UnorganizedCode>();
}
```

Triggers can be derived from deferred inputs with [`.triggers_from()`](bevy_cobweb::NodeBuilder::triggers_from).

```rust
fn derived_trigger(mut web: Web) -> NodeResult<()>
{
    // Make an entity with reactive Score component
    let score_entity = NodeBuilder::new(
            |mut commands: Commands| -> NodeResult<Entity>
            {
                let entity = commands.spawn(React::new(Score)).id();
                Ok(entity)
            }
        )
        .build(&mut web)?;

    // Mutate the score when the IncrementScore event is detected in the environment
    // - Uses a ReactorNode to avoid incrementing the score when the score entity is first set.
    NodeBuilder::new_with(
            score_entity,
            |e| move |mut web: Web, mut score: Query<&mut React<Score>>| -> NodeResult<()>
            {
                let score = score.get_mut(web.read(e)?).map_err(|e| e.into())?;

                score.get_mut(web.rc()).increment();
                Ok(())
            }
        )
        .triggers(event::<IncrementScore>())
        .build_reactor(&mut web)?;

    // React to component mutation
    // - Uses a BasicNode so the score will be printed the first time this is built.
    NodeBuilder::new_with(
            score_entity,
            |e| move |mut web: Web, score: Query<&React<Score>>| -> NodeResult<()>
            {
                let score = score.get(web.read(e)?).map_err(|e| e.into())?;

                println!("Score: {:?}", score);
                Ok(())
            }
        )
        .triggers_from(
            move |web: &Web| -> NodeResult<impl ReactionTriggerBundle>
            {
                Ok(entity_mutation::<Score>(*web.read(score_entity)?))
            }
        )
        .build(&mut web)?;

    Ok(())
}
```


### [NodeHandle\<O\>](bevy_cobweb::NodeHandle) Examples

A node handle can be stored in node state.

```rust
fn handle_into_state(mut web: Web) -> NodeResult<()>
{
    let the_answer = NodeBuilder::new(
            || Ok(42.into()) -> NodeResult<TheAnswer>
        )
        .build(&mut web)?;

    NodeBuilder::new_with(
            the_answer,
            |a| move |mut web: Web| -> NodeResult<()>
            {
                println!("The answer is {:?}", web.read(a)?.proclaim_it());
                Ok(())
            }
        )
        .build(&mut web)?;

    Ok(())
}
```

Or passed as input to a node.

```rust
fn handle_into_input(mut web: Web) -> NodeResult<()>
{
    let the_answer = NodeBuilder::new(
            || Ok(42.into()) -> NodeResult<TheAnswer>
        )
        .build(&mut web)?;

    NodeBuilder::new(
            |mut web: Web, a: NodeInput<NodeHandle<TheAnswer>>| -> NodeResult<()>
            {
                println!("The answer is {:?}", web.read(*a)?.proclaim_it());
                Ok(())
            }
        )
        .input(the_answer)
        .build(&mut web)?;

    Ok(())
}
```

Or used to derive triggers.

```rust
fn handle_into_triggers(mut web: Web) -> NodeResult<()>
{
    let the_man = NodeBuilder::new(
            |mut commands: Commands| Ok(commands.spawn(React::new(TheMan)).id()) -> NodeResult<Entity>
        )
        .build(&mut web)?;

    NodeBuilder::new(
            || -> NodeResult<()>
            {
                println!("The man has spoken!");
                Ok(())
            }
        )
        .triggers_from(
            move |web: &Web| -> NodeResult<impl ReactionTriggerBundle>
            {
                Ok(entity_mutation::<TheMan>(*web.read(the_man)?))
            }
        )
        .build(&mut web)?;

    Ok(())
}
```


### [NodeLink\<O\>](bevy_cobweb::NodeLink) Examples

A node link can pass any arbitrary data using [`.connect()`](bevy_cobweb::NodeBuilder::connect) or [`.connect_merged()`](bevy_cobweb::NodeBuilder::connect_merged) on the consumer.

```rust
fn node_link_connection(mut web: Web) -> NodeResult<()>
{
    let x = 5;
    let y = 10;
    let computed = NodeBuilder::new_with(
            (x, y)
            |(x, y)| move || Ok(x*x + y) -> NodeResult<usize>
        )
        .build_producer(&mut web)?;

    NodeBuilder::connect(
            computed,
            |c| move || -> NodeResult<()>
            {
                println!("The computed result: {c}");
                Ok(())
            }
        )
        .build(&mut web)?;

    Ok(())
}
```

Node links can even pass packaged nodes. The example here is slightly complicated:
1. The bug-spawner node receives node events with the number of bugs to be spawned.
1. Every time it runs, it spawns packaged bug nodes based on the number commanded.
1. The connected node merges the packaged bug nodes into its `BugCache`.
1. The connected node then internally attaches all the packaged bug nodes as children of itself.
1. Finally, we return the bug-spawner node's id so a user of this node can send bug spawn commands as node events to the internal bug-spawner.

If we wanted the parent of `link_with_node_spawning` to send bug spawn commands, then we would need to send the commands to the `link_with_node_spawning` node and then marshal them into the bug-spawner with an internal node event. This would work because the parent has access to the [`NodeId`](bevy_cobweb::NodeId) of `link_with_node_spawning` but not the bug-spawner.

```rust
struct BugCache
{
    saved: Vec<BasicNode<(), (), ()>>,
    pending: Vec<Packaged<BasicNode<(), (), ()>>>,
}
impl Default for BugCache { fn default() -> Self { Self{ saved: Vec::default(), pending: Vec::default() } } }

fn link_with_node_spawning(mut web: Web) -> NodeResult<NodeId>
{
    let new_bugs = NodeBuilder::new(
            |mut web: Web, num: NodeEvent<usize>| -> NodeResult<Vec<Packaged<BasicNode<(), (), ()>>>
            {
                let mut bugs = Vec::default();
                for i in 0..num.take().unwrap_or_default()
                {
                    let bug = NodeBuilder::new(
                            || -> NodeResult<()>
                            {
                                println!("I'm a bug");
                                Ok(())
                            }
                        )
                        .prepare(&mut web)?
                        .packaged(&mut web)?;
                    bugs.push(bug);
                }
                bugs
            }
        )
        .build_producer(&mut web)?;
    let bug_spawner_id = new_bugs.id();

    NodeBuilder::connect_merged(
            new_bugs,
            |cache: Option<BugCache>, new_bugs: Vec<Packaged<BasicNode<(), (), ()>>>| -> MergeResult<BugCache>
            {
                let mut cache = cache.unwrap_or_default();
                cache.pending.append(new_bugs);
                Ok(cache)
            },
            |mut web: Web, mut cache: NodeState<BugCache>| -> NodeResult<()>
            {
                for new_bug in cache.pending.drain()
                {
                    let bug = new_bug.attach(&mut web)?;
                    cache.saved.push(bug);
                }

                for bug in cache.saved.iter()
                {
                    bug.build(&mut web)?;
                }

                Ok(())
            }
        )
        .build(&mut web)?;

    Ok(bug_spawner_id)
}
```


### [NodeHash](bevy_cobweb::NodeHash) Examples

All node state, inputs, and outputs must implement [`NodeHash`](bevy_cobweb::NodeHash) (except [`NodeLinks`](bevy_cobweb::NodeLink)). Types that implement `Hash` also implement `NodeHash` by default.

A custom derive is provided for types whose members all implement `NodeHash`:

```rust
#[derive(NodeHash)]
struct MyNodeOutput
{
    a: usize,
    b: Vec<String>,
    c: NodeHandle<NodeHandle<f32>>,
}
```

You can also implement it manually (very unlikely to need this):

```rust
struct MySpecialNodeOutput
{
    // ???
}

impl NodeHash for MySpecialNodeOutput
{
    fn node_hash(&self, web: &Web, hasher: &mut Hasher)
    {
        // ???
    }
}
```


### Node Event Examples

Node events are quite simple.

```rust
fn basic_event(mut web: Web) -> NodeResult<()>
{
    let id = NodeBuilder::new(
            |event: NodeEvent<()>|
            {
                if event.take().is_some()
                {
                    println!("event encountered!");
                }
                Ok(())
            }
        )
        .build(&mut web)?
        id();

    web.send_event(id, ())?;

    Ok(())
}
```

A major use-case is relocating node branches. In this example, nodes are spawned by a node factory whenever a `Weeble` component is inserted on any entity. The spawned nodes are sent to another node that manages them. The manager node could at a later time package its saved nodes and send them off to another node to be attached there.

Here we also showcase a trick with `from_merged()` to default-initialize node state that isn't hashable.

```rust
fn relocation_event(mut web: Web) -> NodeResult<()>
{
    let id = NodeBuilder::from_merged(
            (),
            |prev, ()| -> MergeResult<Vec<BasicNode<(), (), ()>>>
            {
                Ok(prev.unwrap_or_default())
            },
            |
                mut nodes : NodeState<Vec<BasicNode<(), (), ()>>>,
                event     : NodeEvent<Packaged<BasicNode<(), (), ()>>>
            |
            {
                if let Some(new_node) = event.take()
                {
                    println!("node received!");
                    let node = new_node.attach(&mut web)?;
                    nodes.push(node);
                }

                for node in nodes.iter()
                {
                    node.build(&mut web)?;
                }

                Ok(())
            }
        )
        .build(&mut web)?
        id();

    NodeBuilder::new_with(
            id,
            |id| move |mut web: Web|
            {
                let packaged = NodeBuilder::new(|| {})
                    .prepare(&mut web)?
                    .packaged(&mut web)?;
                web.send(id, packaged);
            }
        )
        .triggers(insertion::<Weeble>())
        .reactor(&mut web)?;

    Ok(())
}
```



## Speculative Examples

Here we imagine how `bevy_cobweb` might be used to build real UIs.


### Score Counter Example

In this example, a game score display increments by 1 every time a game block is destroyed. We assume when blocks are destroyed a `BlockDestroyed` Bevy event is emitted. The score is stored in a `BlockScore` because that's how I would implement this (separation of concerns), however you could also store the score in the node that writes the score text and increment it directly on `BlockDestroyed` events.

The `TextWriter` is a custom system parameter that writes to entities with `Text` components (internally it has a `Query<&mut Text>`). We obtain the text entity in this case from an empty `TextNode` located in the upper right corner of the screen.

```rust
use bevy::prelude::*;
use bevy_cobweb::prelude::*;
use bevy_cobweb_ui::{Location, TextNode, TextSize, TextWriter, WindowArea};

#[derive(ReactResource, Default, Deref, DerefMut)]
struct BlockScore(u32);

#[derive(Event)]
struct BlockDestroyed;

fn score_display(
    mut web : Web,
    window  : Query<Entity, With<PrimaryWindow>>
) -> NodeResult<()>
{
    let area = WindowArea::new(window.single()).build(&mut web)?;

    let text = TextNode::new()
        .location(area, Location::AnchorRelative(1.0, 2.5)
        .size(area, TextSize::RelativeHeight(7.5))
        .build(&mut web)?;

    NodeBuilder::new_with(
            text, |text| move
            |
                web        : Web,
                mut writer : TextWriter,
                score      : ReactRes<BlockScore>
            | -> NodeResult<()>
            {
                writer.write_node(&web, text,
                        |t| write!(t, "Score: {}", *score),
                    )?;
                Ok(())
            }
        )
        .triggers(resource_mutation::<BlockScore>())
        .build(&mut web)?;

    Ok(())
}

fn update_score(
    mut rcommands : ReactCommands,
    mut score     : ReactResMut<BlockScore>,
    mut destroyed : EventReader<BlockDestroyed>
){
    if destroyed.is_empty() { return; }
    let num = destroyed.read().count();
    *score.get_mut(&mut rcommands) += num as u32;
}

fn main()
{
    App::new()
        .add_plugins(DefaultPlugins::default())
        .add_plugins(CobwebPlugin::default())
        .init_react_resource::<BlockScore>()
        .add_event::<BlockDestroyed>()
        //...
        .add_systems(Startup, score_display.webroot())
        .add_systems(Update, update_score)
        //...
        .run();
}
```


### Unit Info Window Sync Example

In this example a unit card is displayed in the upper right corner when a unit is selected. The card updates to reflect the tracked unit's current health, and switches when a new unit is selected.

We assume a unit is selected when the `SelectedUnit` reactive component is added to it. The info card will only display when one entity is selected. If the selected entity has no `React<Health>` component, then a health of `0/0` will be displayed.

```rust
use bevy::prelude::*;
use bevy_cobweb::prelude::*;
use bevy_cobweb_ui::{
    Location, PlainBox, RectDims, TextNode, TextSize, TextWriter, WindowArea
};

#[derive(ReactComponent)]
struct SelectedUnit;

#[derive(ReactComponent, Default, Deref, DerefMut)]
struct Health(u32, u32);

/// Creates a node that translates a node handle of `Option<T>` into `bool`.
fn handle_is_some<T: NodeHash>(web: &mut Web, handle: NodeHandle<Option<T>>) -> NodeResult<bool>
{
    NodeBuilder::new_with(
            handle, |option| move |w: Web| -> NodeResult<bool>
            {
                Ok(w.read(option)?.is_some())
            }
        )
        .build(&mut web)?;
}

/// Creates a unit info card.
fn unit_info_window(
    mut web : Web,
    window  : Query<Entity, With<PrimaryWindow>>
) -> NodeResult<()>
{
    // Detect the selected unit.
    let selected_unit = NodeBuilder::new(
            |s: Query<Entity, With<React<SelectedUnit>>>| -> NodeResult<Option<Entity>>
            {
                let Ok(unit_entity) = s.get_single() else { return Ok(None); };
                Ok(Some(unit_entity))
            }
        )
        .triggers((insertion::<SelectedUnit>(), removal::<SelectedUnit>()))
        .build(&mut web)?;

    // Decide visibility based on a unit being selected.
    let info_visibility = handle_is_some(&mut web, selected_unit)?;

    // Construct the info box.
    let area = WindowArea::new(window.single()).build(&mut web)?;

    let plain_box = PlainBox::new()
        .location(area, Location::ShareAnchor)
        .dimensions(area, RectDims::Relative(10., 7.5))
        .visibility(info_visibility)
        .build(&mut web)?;

    let health_text = TextNode::new()
        .location(plain_box, Location::CenterRelHeight(30.)
        .size(plain_box, TextSize::RelativeHeight(30.))
        .share_visibility(plain_box)
        .build(&mut web)?;

    // Write the health text.
    NodeBuilder::new_with(
            (selected_unit, health_text), |(unit, text)| move
            |
                web        : Web,
                mut writer : TextWriter,
                units      : Query<&React<Health>>
            | -> NodeResult<()>
            {
                let Some(entity) = *web.read(unit)? else { return Ok(()); };
                let health = units.get_single(entity).unwrap_or_default();
                writer.write_node(&web, text,
                        |t| write!(t, "Health: {}/{}", health.0, health.1),
                    )?;
                Ok(())
            }
        )
        .triggers_from(move |web: &Web| -> NodeResult<impl ReactionTriggerBundle>
            {
                let Some(entity) = *web.read(selected_unit)? else { return Ok(()); };
                Ok(entity_mutation::<Health>(entity))
            }
        )
        .build(&mut web)?;

    Ok(())
}

fn main()
{
    App::new()
        .add_plugins(DefaultPlugins::default())
        .add_plugins(CobwebPlugin::default())
        //...
        .add_systems(Startup, unit_info_window.webroot())
        //...
        .run();
}
```


### Exit Confirmation Popup Example

A UI popup that's spawned and absorbed AppExit with a "do you really want to quit"



## `bevy` compatability

| `bevy` | `bevy_cobweb` |
|-------|----------------|
| 0.12  | 0.1 - master   |
