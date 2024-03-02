# Changelog

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
