# Bevy Cobweb

**This crate is under development.**

Bevy Cobweb is a general-purpose reactivity framework for Bevy. It includes core reactivity primitives (invokable systems, system events, reaction triggers, reactive events, and reaction tree processing), and a toolkit for building webs of interconnected, stateful Bevy systems that react to ECS mutations.



## Reactivity Features

- Register systems as [`SystemCommands`](bevy_cobweb::SystemCommand) that are stored on entities.
- System commands can be scheduled manually at any time.
- System commands can react to ECS mutations: resource mutations, component insertions/mutations/removals, entity despawns.
- System commands can react to events: broadcasted events and entity-targeted events.
- System commands can receive data directly via 'system events'.
- System commands, system events, and reactions may all run recursively.
- Reaction trees are processed automatically without any need for manual scheduling.

Documentation for the reactivity primitives can be found [here](src/react/REACT.md).



## Web Features

- Nodes are stateful reactive Bevy systems (carefully designed system commands).
- Node systems run in response to resource mutations, entity changes, reactive events, and node events.
- Node systems also react to changes in their dependencies (inputs) and dependents (their children).
- Node outputs can be accessed throughout the web via node handles that synchronize with rebuilds.
- Change detection prevents reinitializing and rerunning child nodes unless needed.
- Nodes may be detached and re-attached anywhere in the web.
- Nodes are automatically cleaned up when no longer used.
- Node error handling policy is customizable. Errors propagate to root nodes.
- Node reactions are processed immediately and recursion is allowed.

Documentation for the web toolkit can be found [here](src/web/WEB.md).



## UI Implementations

This crate is designed to be a foundation for building UI in Bevy, with the following goals in mind.

- Build UI declaratively in a Bevy-native style using normal-looking Bevy systems.
- Unify UI element construction and mutation, with change detection to avoid updating unless necessary.
- Enable asset- and editor-driven hot-reloading that preserves the existing UI state as much as possible.
- Provide a powerful, unopinionated API for building ergonomic UI widgets.
- Enable highly-responsive multi-widget structures that react to ECS mutations and automatically update when internal dependencies change.
- No macros required, no third-party dependencies.

UI libraries based on `bevy_cobweb` are:
- `bevy_cobweb_ui`: TODO



## `bevy` compatability

| `bevy` | `bevy_cobweb` |
|-------|----------------|
| 0.12  | 0.1 - master   |
