# Reactivity Primitives

Reactivity is built on system commands, system events, a core reactivity API, and a custom system runner.


## System Commands

All reactors are [`SystemCommands`](bevy_cobweb::prelude::SystemCommand).


### Spawning Systems

Systems can be spawned as [`SystemCommands`](bevy_cobweb::prelude::SystemCommand) with [`Commands::spawn_system_command`](bevy_cobweb::prelude::ReactCommandsExt::spawn_system_command). System commands are similar to Bevy one-shot systems, however the actual system is wrapped in a closure that takes `World` and a [`SystemCommandCleanup`](bevy_cobweb::prelude::SystemCommandCleanup) as input. See [Custom command runner](#Custom-command-runner) for more details.

Example:
```rust
let syscommand = commands.spawn_system_command(
    |weebles: Res<Weebles>|
    {
        println!("there are {} weebles", weebles.num());
    }
);
```

System commands return anything that implements [`CobwebResult`](bevy_cobweb::prelude::CobwebResult). This includes `()`, [`DropErr`](bevy_cobweb::prelude::DropErr), and [`WarnErr`](bevy_cobweb::prelude::WarnErr). `DropErr` lets you early-out from systems using `?`. It just requires [`DONE`](bevy_cobweb::prelude::DONE) at the end of the system. Similarly, `WarnErr` requires [`OK`](bevy_cobweb::prelude::OK) at the end of the system.

If this example errors, the error will be silently dropped.
```rust
let syscommand = commands.spawn_system_command(
    |weebles: Query<&Weebles>|
    {
        let weeble = weebles.get_single()?;
        println!("my weeble: {:?}", weeble);
        DONE
    }
);
```

If this example errors, the error will be logged as a warning.
```rust
let syscommand = commands.spawn_system_command(
    |weebles: Query<&Weebles>|
    {
        let weeble = weebles.get_single()?;
        println!("my weeble: {:?}", weeble);
        OK
    }
);
```

### Running System Commands

A [`SystemCommand`](bevy_cobweb::prelude::SystemCommand) can be manually run by scheduling it as a Bevy `Command`.

```rust
commands.queue(syscommand);
```



## System Events

You can send data directly to a system spawned as a [`SystemCommand`](bevy_cobweb::prelude::SystemCommand) by sending it a system event. The biggest advantage here is being able to move data into a system.

For example, using the [`SystemEvent`](bevy_cobweb::prelude::SystemEvent) system parameter to consume the event data:
```rust
let syscommand = commands.spawn_system_command(
    |mut data: SystemEvent<Vec<u32>>|
    {
        let data = data.take()?;
        for val in data
        {
            println!("recieved {}", val);
        }
        DONE
    }
);

commands.send_system_event(syscommand, vec![0, 18, 42]);
```



## Reactivity API

ECS reactivity is only implemented for [`ReactResource`](bevy_cobweb::prelude::ReactResource) resources and [`ReactComponent`](bevy_cobweb::prelude::ReactComponent) components, which are accessed with [`ReactRes`](bevy_cobweb::prelude::ReactRes)/[`ReactResMut`](bevy_cobweb::prelude::ReactResMut) system parameters and the [`React<C>`](bevy_cobweb::prelude::React) component wrapper (or [`Reactive<C>`](bevy_cobweb::prelude::Reactive)/[`ReactiveMut<C>`](bevy_cobweb::prelude::ReactiveMut) system parameters) respectively.

We use `ReactResource`/`ReactComponent` instead of Bevy change detection in order to achieve precise, responsive, recursive reactions with an ergonomic API. Bevy implemented [observers and hooks](https://github.com/bevyengine/bevy/pull/10839) in v0.14, however there is no hook for resource or component mutation. We may use observers under the hood to improve performance in some areas, but overall we expect the current `bevy_cobweb` API to stay the same indefinitely. Note that Bevy observers currently run around 6-10x faster than `bevy_cobweb` reactors, although this is a difference of `300ns` vs `50ns` per reaction which is insignificant for common workloads (e.g. UI interactions, low/medium-frequency resource updates, etc.). For workloads on the hot path we recommend using normal Bevy systems.

Reactors are run in-line in the Bevy `Commands` execution flow, which means they will naturally telescope if reactions are triggered from inside other reactors. For more details see [Custom command runner](#Custom-command-runner).


### Registering Reactors

Reactors are registered with [`ReactCommands`](bevy_cobweb::prelude::ReactCommands), which are obtained from [`Commands::react`](ReactCommandsExt::react). You must specify a 'reaction trigger':
```rust
fn setup(mut c: Commands)
{
    c.react().on(resource_mutation::<A>(),
        |a: ReactRes<A>|
        {
            //...
        }
    );
}
```

The available reaction triggers are:
- [`resource_mutation<R: ReactResource>`](bevy_cobweb::prelude::resource_mutation)
- [`insertion<C: ReactComponent>`](bevy_cobweb::prelude::insertion)
- [`mutation<C: ReactComponent>`](bevy_cobweb::prelude::mutation)
- [`removal<C: ReactComponent>`](bevy_cobweb::prelude::removal)
- [`entity_insertion<C: ReactComponent>`](bevy_cobweb::prelude::entity_insertion)
- [`entity_mutation<C: ReactComponent>`](bevy_cobweb::prelude::entity_mutation)
- [`entity_removal<C: ReactComponent>`](bevy_cobweb::prelude::entity_removal)
- [`despawn`](bevy_cobweb::prelude::despawn)
- [`broadcast<E>`](bevy_cobweb::prelude::broadcast)
- [`entity_event<E>`](bevy_cobweb::prelude::entity_event)
- [`any_entity_event<E>`](bevy_cobweb::prelude::any_entity_event)

A reactor can be associated with multiple reaction triggers:
```rust
fn setup(mut c: Commands)
{
    c.react().on((resource_mutation::<A>(), entity_insertion::<B>(entity)),
        move |a: ReactRes<A>, q: Query<&React<B>>|
        {
            q.get(entity);
            //...etc.
        }
    );
}
```


### Revoking Reactors

Reactors can be revoked with [`RevokeTokens`](bevy_cobweb::prelude::RevokeToken) obtained on registration.

```rust
let token = c.react().on_revokable(resource_mutation::<A>(), || { todo!(); });
c.react().revoke(token);
```


### Trigger Type: Resource Mutation

Add a reactive resource to your app:
```rust
#[derive(ReactResource, Default)]
struct Counter(u32);

app.add_plugins(ReactPlugin)
    .init_react_resource::<Counter>();
```

Mutate the resource:
```rust
fn increment(mut c: Commands, mut counter: ReactResMut<Counter>)
{
    counter.get_mut(&mut c).0 += 1;
}
```

React to the resource mutation:
```rust
fn setup(mut c: Commands)
{
    c.react().on(resource_mutation::<Counter>(),
        |counter: ReactRes<Counter>|
        {
            println!("count: {}", counter.0);
        }
    );
}
```


### Trigger Type: Component Insertion/Mutation/Removal

A reactor can listen to component insertion/mutation/removal on *any* entity or a *specific* entity. In either case, the reactor can read which entity the event occurred on with the [`InsertionEvent`](bevy_cobweb::prelude::InsertionEvent), [`MutationEvent`](bevy_cobweb::prelude::MutationEvent), and [`RemovalEvent`](bevy_cobweb::prelude::RemovalEvent) system parameters.

```rust
#[derive(ReactComponent)]
struct Health(u16);

fn setup(mut c: Commands)
{
    // On any entity.
    c.react().on(insertion::<Health>(),
        |event: InsertionEvent<Health>, q: Query<&React<Health>>|
        {
            let entity = event.try_read()?;
            let health = q.get(entity)?;
            println!("new health: {}", health.0);
            DONE
        }
    );

    // On a specific entity.
    let entity = c.spawn_empty().id();
    c.react().on(entity_mutation::<Health>(entity),
        |event: InsertionEvent<Health>, q: Query<&React<Health>>|
        {
            let entity = event.try_read()?;
            let health = q.get(entity)?;
            println!("updated health: {}", health.0);
            DONE
        }
    );

    // Trigger the insertion reactors.
    c.react().insert(entity, Health(0u16));
}

fn add_health(mut c: Commands, mut q: Query<&mut React<Health>>)
{
    for health in q.iter_mut()
    {
        health.get_mut(&mut c).0 += 10;
    }
}
```


### Trigger Type: Despawns

React to a despawn, using the [`DespawnEvent`](bevy_cobweb::prelude::DespawnEvent) system parameter to read which entity was despawned:
```rust
c.react().on(despawn(entity),
    |entity: DespawnEvent|
    {
        println!("entity despawned: {}", entity.read());
    }
);
```


### Trigger Type: Broadcast Events

Send a broadcast:
```rust
c.react().broadcast(0u32);
```

React to the event, using the [`BroadcastEvent`](bevy_cobweb::prelude::BroadcastEvent) system parameter to access event data:
```rust
c.react().on(broadcast::<u32>(),
    |event: BroadcastEvent<u32>|
    {
        let event = event.try_read()?:
        println!("broadcast: {}", event);

        DONE
    }
);
```


### Trigger Type: Entity Events

Entity events can be considered 'scoped broadcasts', sent only to systems listening to the target entity. If the target entity is despawned, then entity events targeting it will be dropped.

Send an entity event:
```rust
c.react().entity_event(entity, 0u32);
```

React to the event, using the [`EntityEvent`](bevy_cobweb::prelude::EntityEvent) system parameter to access event data:
```rust
c.react().on(entity_event::<u32>(entity),
    |event: EntityEvent<u32>|
    {
        let (entity, event) = event.try_read()?;
        println!("entity: {:?}, event: {}", entity, event);

        DONE
    }
);
```


### One-off Reactors

If you only want a reactor to run at most once, use [`ReactCommands::once`]:
```rust
let entity = c.spawn(Player);
c.react().once(broadcast::<ResetEverything>(),
    move |world: &mut World|
    {
        world.despawn(entity);
    }
);
```


### Reactor Cleanup

Reactors are stateful boxed Bevy systems, so it is useful to manage their memory use. We control reactor lifetimes with [`ReactorMode`](bevy_cobweb::prelude::ReactorMode), which has three settings. You can manually specify the mode using [`ReactCommands::with`](bevy_cobweb::prelude::ReactCommands::with).

- [`ReactorMode::Persistent`](bevy_cobweb::prelude::ReactorMode::Persistent): The reactor will never be cleaned up even if it has no triggers. This is the most efficient mode because there is no need to allocate a despawn counter or revoke token.
    - See [`ReactCommands::on_persistent`](bevy_cobweb::prelude::ReactCommands::on_persistent), which returns a [`SystemCommand`](bevy_cobweb::prelude::SystemCommand).
- [`ReactorMode::Cleanup`](bevy_cobweb::prelude::ReactorMode::Cleanup): The reactor will be cleaned up if it has no triggers, including if it started with [`despawn`](bevy_cobweb::prelude::despawn) triggers and all despawns have fired.
    - See [`ReactCommands::on`](bevy_cobweb::prelude::ReactCommands::on).
- [`ReactorMode::Revokable`](bevy_cobweb::prelude::ReactorMode::Revokable): The reactor will be cleaned up if it has no triggers, including if it starts with [`despawn`](bevy_cobweb::prelude::despawn) triggers and all despawns have fired. Otherwise, you can revoke it manually with its [`RevokeToken`](bevy_cobweb::prelude::RevokeToken) and [`ReactCommands::revoke`](bevy_cobweb::prelude::ReactCommands::revoke).
    - See [`ReactCommands::on_revokable`](bevy_cobweb::prelude::ReactCommands::on_revokable), which returns a [`RevokeToken`](bevy_cobweb::prelude::RevokeToken).


### World Reactors

Special [`WorldReactors`](bevy_cobweb::prelude::WorldReactor) can be registered with apps and accessed with the [`Reactor<T: WorldReactor>`](bevy_cobweb::prelude::Reactor) system parameter. World reactors are similar to Bevy systems in that they live for the entire lifetime of an app.

The advantage of world reactors over normal reactors is you can easily add/remove triggers from them anywhere in your app. You can also easily run them manually from anywhere in your app. They also only need to be allocated once, as opposed to normal reactors that must be boxed every time you register one (and then their internal system state needs to be initialized).

Define a [`WorldReactor`](bevy_cobweb::prelude::WorldReactor):
```rust
#[derive(ReactComponent)]
struct A;

struct DemoReactor;

impl WorldReactor for DemoReactor
{
    type StartingTriggers = InsertionTrigger<A>;
    type Triggers = EntityMutationTrigger<A>;

    fn reactor(self) -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            |insertion: InsertionEvent<A>, mutation: MutationEvent<A>|
            {
                if let Some(_) = insertion.try_read()
                {
                    println!("A was inserted on an entity");
                }
                if let Some(_) = mutation.try_read()
                {
                    println!("A was mutated on an entity");
                }
            }
        )
    }
}
```

Add the reactor to your app:
```rust
fn setup(app: &mut App)
{
    app.add_world_reactor_with(DemoReactor, mutation::<A>());
}
```

Add a trigger to the reactor:
```rust
fn spawn_a(mut c: Commands, mut reactor: Reactor<DemoReactor>)
{
    let entity = c.spawn_empty().id();
    c.react().insert(entity, A);
    reactor.add(&mut c, entity_mutation::<A>(entity));
}
```


### Entity World Reactors

Similar to [`WorldReactor`](bevy_cobweb::prelude::WorldReactor) is [`EntityWorldReactor`](bevy_cobweb::prelude::EntityWorldReactor), which is used for entity-specific reactors (entity component insertion/mutation/removal and entity events). For each entity that is tracked by the reactor, you can add [`EntityWorldReactor::Local`](bevy_cobweb::prelude::EntityWorldReactor::Local) data that is readable/writable with [`EntityLocal`](bevy_cobweb::prelude::EntityLocal) when that entity triggers a reaction.

Adding an entity to an entity world reactor will register that reactor to run whenever the triggers in [`EntityWorldReactor::Triggers`](bevy_cobweb::prelude::EntityWorldReactor::Triggers) are activated on that entity. You don't need to manually specify the triggers.

In the following example, we write the time to a reactive component every 500ms. The reactor picks this up and prints a message tailored to the reacting entity.

```rust
#[derive(ReactComponent, Eq, PartialEq)]
struct TimeRecorder(Duration);

struct TimeReactor;
impl EntityWorldReactor for TimeReactor
{
    type Triggers = EntityMutationTrigger<TimeRecorder>;
    type Local = String;

    fn reactor() -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            |data: EntityLocal<TimeReactor>, components: Reactive<TimeRecorder>|
            {
                let (entity, data) = data.get();
                let component = components.get(entity)?;
                println!("Entity {:?} now has {:?}", data, component);
            }
        )
    }
}

fn prep_entity(mut c: Commands, reactor: EntityReactor<TimeReactor>)
{
    let entity = c.spawn(TimeRecorder(Duration::default()));
    reactor.add(&mut c, entity, "ClockTracker");
}

fn update_entity(mut commands: Commands, time: Res<Time>, mut components: ReactiveMut<TimeRecorder>)
{
    components.set_single_if_not_eq(&mut c, TimeRecorder(time.elapsed()));
}

struct ExamplePlugin;
impl Plugin for ExamplePlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_entity_reactor::<TimeReactor>()
            .add_systems(Setup, prep_entity)
            .add_systems(Update, update_entity.run_if(on_timer(Duration::from_millis(500))));
    }
}
```



## Custom command runner

We use a custom system command runner to run `bevy_cobweb` reactors and system commands. This allows us to insert cleanup logic between when the system runs and when its internally deferred commands are executed.

In the current design we include entity garbage collection and component-removal and despawn reactor scheduling within the system command runner.

1. Garbage collect [`AutoDespawner`](bevy_cobweb::prelude::AutoDespawner) entities and schedule component-removal and despawn reactions.
1. Remove the target system command from the `World`.
    1. If the system is missing, run the cleanup callback and return.
1. Run the system command. Internally this does the following:
    1. Run the system on the world: `system.run((), world)`.
    1. Invoke the cleanup callback.
    1. Apply deferred: `system.apply_deferred(world)`.
1. Garbage collect [`AutoDespawner`](bevy_cobweb::prelude::AutoDespawner) entities. Including this ensures if a system command garbage collected itself, the insertion-point will be gone so the system state will be dropped.
1. Reinsert the system command into the `World`.
1. Garbage collect [`AutoDespawner`](bevy_cobweb::prelude::AutoDespawner) entities and schedule component-removal and despawn reactions.

- **Injected cleanup**: In `bevy_cobweb` you access reactive event data with the [`InsertionEvent`](bevy_cobweb::prelude::InsertionEvent), [`MutationEvent`](bevy_cobweb::prelude::MutationEvent), [`RemovalEvent`](bevy_cobweb::prelude::RemovalEvent), [`DespawnEvent`](bevy_cobweb::prelude::DespawnEvent), [`BroadcastEvent`](bevy_cobweb::prelude::BroadcastEvent), [`EntityEvent`](bevy_cobweb::prelude::EntityEvent), and [`SystemEvent`](bevy_cobweb::prelude::SystemEvent) system parameters. In order to properly set the underlying data of these parameters such that future system calls won't accidentally have access to that data, our strategy is to insert the data to custom resources and entities immediately before running [`SystemCommands`](bevy_cobweb::prelude::SystemCommand) and then remove that data immediately after the system has run but before calling `apply_deferred`. We do this with an injected cleanup callback in the system runner ([`SystemCommandCleanup`](bevy_cobweb::prelude::SystemCommandCleanup)).


### Recursive system commands

It is allowed for a system command to recursively schedule itself to run (or e.g. for a reactor to trigger itself), *however* recursive systems *do not* run in-line with other commands. Instead we extract them into a queue and run them after their duplicate ancestor has been re-inserted to its entity.

In general, it is not recommended to use recursive system commands because the control flow becomes very convoluted, which makes code fragile and bug-prone.
