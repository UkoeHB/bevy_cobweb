# Changelog

## [0.0.8]

### Changed

- `AutoDespawner` now uses `despawn_recursive`.
- Optimized entity-specific reactors.
- Moved entity event reactor handles so they are stored on entities, ensuring they are cleaned up automatically when entities despawn.

### Added

- Impl `From<RevokeToken>` for `SystemCommand`.
- Added `ReactorMode` for more versatile and efficient reactor management.


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
