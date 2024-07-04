# Changelog

## [0.8.1]

### Fixed

- Callback system cleanup now properly runs between the system and when its commands are flushed/applied.


## [0.8.0]

- Update to Bevy v0.14.


## [0.7.2]

### Added

- Added `.react()` extension method to `App` and `World`.


## [0.7.1]

### Added

- Implement `CommandsSyscallExt` for `EntityCommands`.


## [0.7.0]

### Changed

- `set_if_not_eq` -> `set_if_neq`
- Removed `bevy_fn_plugin` dependency.

### Fixed

- Avoid redundant despawns, which cause Bevy error B0003.


## [0.6.0]

### Added

- Added `ReactiveMut::set_single_if_not_eq`.
- Added `.react()` extension method for `EntityCommands`.

### Changed

- The `Reactive` and `ReactiveMut` system param's 'single' methods now return the single entity's id.


## [0.5.0]

### Added

- Added `EntityWorldReactor` for entity-associated reactions, with `EntityLocal` that can read per-entity custom data in reactors.


## [0.4.0]

### Added

- Added `Reactive` and `ReactiveMut` system parameters for easier access to `React` components.

### Changed

- Rename `*_mut_noreact` to `*_noreact` for simplicity.
- Rework `ReactCommands` to be derived from `Commands` instead of its own system parameter. Add `Commands::react` extension method for obtaining `ReactCommands` instances.


## [0.3.0]

### Added

- Added `AnyEntityEventTrigger` with associated `any_entity_event` helper method.

### Changed

- `EntityEvent::read()` now returns `Option<(Entity, &T)>` instead of `&Option<(Entity, T)>.


## [0.2.2]

### Changed

- Loosen `set_if_not_eq` requirement from `Eq` to `PartialEq`.


## [0.2.1]

### Changed

- Panic if duplicate world reactors are added to an app.

### Added

- Added `broadcast` and `entity_event` methods to `ReactWorldExt`.


## [0.2.0]

### Changed

- Rename `SystemCommandCallback::from_system` -> `SystemCommandCallback::new` and `SystemCommandCallback::new` -> `SystemCommandCallback::with`.
- Rename `BroadcastEventTrigger` -> `BroadcastTrigger`.

### Added

- Added `WorldReactor` trait with `Reactor` system param.
- Added `ReactAppExt` and `ReactWorldExt`.


## [0.1.0]

### Changed

- `AutoDespawner` now uses `despawn_recursive`.
- Optimized entity-specific reactors.
- Moved entity event reactor handles so they are stored on entities, ensuring they are cleaned up automatically when entities despawn.
- Component removal reactors are now triggered even if the entity was despawned. This matches Bevy's `RemovedComponents` behavior.

### Added

- Impl `From<RevokeToken>` for `SystemCommand`.
- Added `ReactorMode` for more versatile and efficient reactor management.

### Removed

- Removed docs and files related to the 'reactive web' concept, which will not be pursued.


## [0.0.7]

### Added

- `ReactCommands::register_and_run_once`

### Fixed

- All `ReactCommands` actions are now deferred to ensure there is no partial mutation of the react state when a given command is applied.


## [0.0.6]

### Fixed

- Remove `syscall` from `SpawnedSyscallCommandsExt`.


## [0.0.5]

### Added

- Add `Commands::syscall` for scheduling system calls from within systems.


## [0.0.4]

### Changed

- Update to Bevy v0.13


## [0.0.3]

### Fixed

- Bug where reactive events were being processed before the event data was spawned.


## [0.0.2]

### Changed

- Add reactivity primitives.


## 0.0.1

- Reserve crates.io name.
