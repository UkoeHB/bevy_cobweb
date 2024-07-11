# Bevy Cobweb

General-purpose reactivity framework for Bevy. Includes ECS utilities and core reactivity primitives (system events, reaction triggers, reactive events, and reaction tree processing).

Documentation for the reactivity primitives can be found in the [`react`](bevy_cobweb::react) module.



## Features

- Manually run systems with [`SystemCommands`](bevy_cobweb::prelude::SystemCommand).
- React to ECS mutations: resource mutations, component insertions/mutations/removals, entity despawns.
- React to events: broadcasted events and entity-targeted events.
- Send data directly to systems with system events.
- Write recursive system commands/system events/reactions.



## Companion crates

- [bevy_cobweb_ui](https://github.com/UkoeHB/bevy_cobweb_ui): Reactive UI framework.



## Bevy compatability

| `bevy` | `bevy_cobweb` |
|-------|----------------|
| 0.14  | 0.8.0 - master |
| 0.13  | 0.0.3 - 0.7.2  |
| 0.12  | 0.0.1 - 0.0.2  |
