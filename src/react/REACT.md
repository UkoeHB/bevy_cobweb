# Reactivity Primitives

Reactivity is built on system commands, system events, a core reactivity API, and a custom scheduling algorithm.


## System Commands

All reactors are [`SystemCommands`](bevy_cobweb::prelude::SystemCommand).


### Spawning Systems

Systems can be spawned as [`SystemCommands`](bevy_cobweb::prelude::SystemCommand) with [`Commands::spawn_system_command`](bevy_cobweb::prelude::ReactCommandsExt::spawn_system_command). System commands are similar to Bevy one-shot systems, however the actual system is wrapped in a closure that takes `World` and a [`SystemCommandCleanup`](bevy_cobweb::prelude::SystemCommandCleanup) as input. See [Scheduling](#scheduling) for more details.

Example:
```rust
let syscommand = commands.spawn_system_command(
    |weebles: Res<Weebles>|
    {
        println!("there are {} weebles", weebles.num());
    }
);
```


### Running System Commands

A [`SystemCommand`](bevy_cobweb::prelude::SystemCommand) can be manually run by scheduling it as a Bevy `Command`. Scheduling a system command will cause a reaction tree to run (see [Scheduling](#scheduling)).

```rust
commands.add(syscommand);
```



## System Events

You can send data directly to a system spawned as a [`SystemCommand`](bevy_cobweb::prelude::SystemCommand) by sending it a system event.

For example, using the [`SystemEvent`](bevy_cobweb::prelude::SystemEvent) system parameter to consume the event data:
```rust
let syscommand = commands.spawn_system_command(
    |mut data: SystemEvent<Vec<u32>>|
    {
        let Some(data) = data.take() else { return; };
        for val in data
        {
            println!("recieved {}", val);
        }
    }
);

commands.send_system_event(syscommand, vec![0, 18, 42]);
```

Sending a system event will cause a reaction tree to run (see [Scheduling](#scheduling)).



## Reactivity API

ECS reactivity is only implemented for [`ReactResource`](bevy_cobweb::prelude::ReactResource) resources and [`ReactComponent`](bevy_cobweb::prelude::ReactComponent) components, which are accessed with [`ReactRes`](bevy_cobweb::prelude::ReactRes)/[`ReactResMut`](bevy_cobweb::prelude::ReactResMut) system parameters and the [`React<C>`](bevy_cobweb::prelude::React) component wrapper respectively.

We use `ReactResource`/`ReactComponent` instead of Bevy change detection in order to achieve precise, responsive, recursive reactions with an ergonomic API. When Bevy implements [observers](https://github.com/bevyengine/bevy/pull/10839), we expect the 'extra' API layer to be eliminated.

A reactor will run in the first `apply_deferred` after its reaction trigger is detected. If a reactor triggers other reactors, they will run immediately after the initial reactor in a telescoping fashion until the entire tree of reactions terminates. Recursive reactions are fully supported. For more details see [Scheduling](#scheduling).


### Registering Reactors

Reactors are registered with [`ReactCommands`](bevy_cobweb::prelude::ReactCommands). You must specify a 'reaction trigger':
```rust
fn setup(mut rcommands: ReactCommands)
{
    rcommands.on(resource_mutation::<A>(),
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

A reactor can be associated with multiple reaction triggers:
```rust
fn setup(mut rcommands: ReactCommands)
{
    rcommands.on((resource_mutation::<A>(), entity_insertion::<B>(entity)),
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
let token = rcommands.on(resource_mutation::<A>(), || { todo!(); });
rcommands.revoke(token);
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
fn increment(mut rcommands: ReactCommands, mut counter: ReactResMut<Counter>)
{
    counter.get_mut(&mut rcommands).0 += 1;
}
```

React to the resource mutation:
```rust
fn setup(mut rcommands: ReactCommands)
{
    rcommands.on(resource_mutation::<Counter>(),
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

fn setup(mut rcommands: ReactCommands)
{
    // On any entity.
    rcommands.on(insertion::<Health>(),
        |event: InsertionEvent<Health>, q: Query<&React<Health>>|
        {
            let Some(entity) = event.read() else { return; };
            let health = q.get(entity).unwrap();
            println!("new health: {}", health.0);
        }
    );

    // On a specific entity.
    let entity = rcommands.commands().spawn_empty().id();
    rcommands.on(entity_mutation::<Health>(entity),
        |event: InsertionEvent<Health>, q: Query<&React<Health>>|
        {
            let Some(entity) = event.read() else { return; };
            let health = q.get(entity).unwrap();
            println!("updated health: {}", health.0);
        }
    );

    // Trigger the insertion reactors.
    rcommands.insert(entity, Health(0u16));
}

fn add_health(mut rcommands: ReactCommands, mut q: Query<&mut React<Health>>)
{
    for health in q.iter_mut()
    {
        health.get_mut(&mut rcommands).0 += 10;
    }
}
```


### Trigger Type: Despawns

React to a despawn, using the [`DespawnEvent`](bevy_cobweb::prelude::DespawnEvent) system parameter to read which entity was despawned:
```rust
rcommands.on(despawn(entity),
    |entity: DespawnEvent|
    {
        println!("entity despawned: {}", entity.read().unwrap());
    }
);
```


### Trigger Type: Broadcast Events

Send a broadcast:
```rust
rcommands.broadcast(0u32);
```

React to the event, using the [`BroadcastEvent`](bevy_cobweb::prelude::BroadcastEvent) system parameter to access event data:
```rust
rcommands.on(broadcast::<u32>(),
    |event: BroadcastEvent<u32>|
    {
        if let Some(event) = event.read()
        {
            println!("broadcast: {}", event);
        }
    }
);
```


### Trigger Type: Entity Events

Entity events can be considered 'scoped broadcasts', sent only to systems listening to the target entity. If the target entity is despawned, then entity events targeting it will be dropped.

Send an entity event:
```rust
rcommands.entity_event(entity, 0u32);
```

React to the event, using the [`EntityEvent`](bevy_cobweb::prelude::EntityEvent) system parameter to access event data:
```rust
rcommands.on(entity_event::<u32>(entity),
    |event: EntityEvent<u32>|
    {
        if let Some((entity, event)) = event.read()
        {
            println!("entity: {:?}, event: {}", entity, event);
        }
    }
);
```


### One-off Reactors

If you only want a reactor to run once, use [`ReactCommands::once`]:
```rust
let entity = rcommands.commands().spawn(Player);
rcommands.once(broadcast::<ResetEverything>(),
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
- [`ReactorMode::Revokable`](bevy_cobweb::prelude::ReactorMode::Revokable): The reactor will be cleaned up if it has no triggers, including if it starts with [`despawn`](bevy_cobweb::prelude::despawn) triggers and all despawns have fired. Otherwise, you can revoke it manually with its [`RevokeToken`](bevy_cobweb::prelude::RevokeToken).
    - See [`ReactCommands::on_revokable`](bevy_cobweb::prelude::ReactCommands::on_revokable), which returns a [`RevokeToken`](bevy_cobweb::prelude::RevokeToken).


### World Reactors

Special [`WorldReactors`](bevy_cobweb::prelude::WorldReactor) can be registered with apps and accessed with the [`Reactor<T: WorldReactor>`](bevy_cobweb::prelude::Reactor) system parameter. World reactors are similar to Bevy systems in that they live for the entire lifetime of an app. The advantage of world reactors is you can easily add/remove triggers from them without needing to create completely new reactors (which requires allocation). You can also easily run them manually from anywhere in your app.

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
                if let Some(_) = insertion.read()
                {
                    println!("A was inserted on an entity");
                }
                if let Some(_) = mutation.read()
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
    app.add_reactor(DemoReactor);
}
```

Add a trigger to the reactor:
```rust
fn spawn_a(mut rc: ReactCommands, mut reactor: Reactor<DemoReactor>)
{
    let entity = rc.commands().spawn_empty().id();
    rc.insert(A, entity);
    reactor.add_triggers(&mut rc, entity_mutation::<A>(entity));
}
```



## Scheduling

In order to support recursive reactions and system events, `bevy_cobweb` extends Bevy's simple `Commands` feature by adding additional command-like scheduling, resulting in a 4-tier structure. Processing all of those tiers requires a custom scheduling algorithm, which we discuss below.


### Commands

Conceptually, the four tiers are as follows:

1. Inner-system commands (`Commands`): Single-system ECS mutations and system-specific deferred logic.
1. System commands ([`SystemCommand`](bevy_cobweb::prelude::SystemCommand)): Execution of a single system. One [`SystemCommand`](bevy_cobweb::prelude::SystemCommand) can schedule further system commands, which can be considered 'extensions' of their parent in a functional-programming sense.
1. System events (`EventCommand`): Sending data to a system which triggers it to run. System events scheduled by other system events are then considered follow-up actions, rather than extensions of the originating event.
1. Reactions (`ReactionCommand`): ECS mutations or reactive events that trigger a system to run. A single reaction may result in a single system running, a cascade of system commands, or a cascade of system commands followed by a series of system events. Reactions may also trigger other reactions, which will run after the previous reaction has fully resolved itself (after all system commands and events have been recursively processed).

Each tier expands in a telescoping fashion. When one `Command` is done running, all commands queued by that `Command` are immediately executed before any previous commands, and so on for the other tiers.

**Telescoping Caveat**

Reaction trees are often triggered within normal Bevy systems by ECS mutations/events/etc. These trees will therefore run at a specific point in the command queues of the normal Bevy systems that trigger them, rather than waiting until the end of the queue.


### Innovations

There are two important innovations that the `bevy_cobweb` command-resolver algorithm introduces.
- **Rearranged `apply_deferred`**:
    - **The problem**: Any Bevy system can have internally deferred logic that is saved in system parameters. After a system runs, that deferred logic can be applied by calling `system.apply_deferred(&mut world)`. The problem with this is if the deferred logic includes triggers to run the same system again (e.g. because of reactivity), an error will occur because the system is currently in use.
    - **The solution**: To solve this, `bevy_cobweb` only uses `apply_deferred` to apply the first command tier. Everything else is executed after the system has been returned to the world.
- **Injected cleanup**: In `bevy_cobweb` you access reactive event data with the [`InsertionEvent`](bevy_cobweb::prelude::InsertionEvent), [`MutationEvent`](bevy_cobweb::prelude::MutationEvent), [`RemovalEvent`](bevy_cobweb::prelude::RemovalEvent), [`DespawnEvent`](bevy_cobweb::prelude::DespawnEvent), [`BroadcastEvent`](bevy_cobweb::prelude::BroadcastEvent), [`EntityEvent`](bevy_cobweb::prelude::EntityEvent), and [`SystemEvent`](bevy_cobweb::prelude::SystemEvent) system parameters. In order to properly set the underlying data of these parameters such that future system calls won't accidentally have access to that data, our strategy is to insert the data to custom resources and entities immediately before running [`SystemCommands`](bevy_cobweb::prelude::SystemCommand) and then remove that data immediately after the system has run but before calling `apply_deferred`. We do this with an injected cleanup callback in the system runner ([`SystemCommandCleanup`](bevy_cobweb::prelude::SystemCommandCleanup)).


### Scheduler Algorithm

The scheduler has two pieces. Note that all systems in this context are custom one-shot systems stored on entities.

In order to rearrange `apply_deferred` as described, all system commands, system events, and reactions are queued within internal `CobwebCommandQueue` resources.

**1. System command runner**

At the lowest level is the system command runner, which executes a single scheduled system command. All Bevy `Commands` and system commands created by the system that is run will be resolved here.

1. Remove the target system command from the `World`.
    1. If the system is missing, run the cleanup callback and return.
1. Remove pre-existing pending system commands.
1. Run the system command. Internally this does the following:
    1. Run the system on the world: `system.run((), world)`.
    1. Invoke the cleanup callback.
    1. Apply deferred: `system.apply_deferred(world)`.
1. Reinsert the system command into the `World`.
1. Take pending system commands and run them with this system runner. Doing this will automatically cause system command telescoping.
1. Replace pre-existing pending system commands that were removed.

**2. Reaction tree**

Whenever a system command, system event, or reaction is scheduled, we schedule a normal Bevy `Command` that launches a reaction tree. The reaction tree will early-out if a reaction tree is already being processed.

The reaction tree will fully execute all recursive system commands, system events, and reactions before returning. The algorithm is as follows:

1. Set the reaction tree flag to prevent the tree from being recursively executed.
1. Remove existing system events and reactions.
1. Loop until there are no pending system commands, system events, or reactions.
    1. Loop until there are no pending system commands or system events.
        1. Loop until there are no pending system commands.
            1. Pop one system command from the queue and run it with the system runner. This will internally telescope.
        1. Remove pending system events and push them to the front of the system events queue.
        1. Pop one system event from the queue and run it.
    1. Remove pending reactions and push them to the front of the reactions queue.
    1. Pop one reaction from the queue and run it.
1. Unset the reaction tree flag now that everything has been processed.
