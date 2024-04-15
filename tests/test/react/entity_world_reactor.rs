//local shortcuts
use bevy_cobweb::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Reactor with no starting triggers.
struct EmptyReactor(Arc<AtomicU32>);

impl EntityWorldReactor for EmptyReactor
{
    type StartingTriggers = ();
    type Triggers = EntityEventTrigger<usize>;
    type Data = ();

    fn reactor(self) -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            move ||
            {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        )
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Reactor with starting triggers.
struct StartingReactor(Arc<AtomicU32>);

impl EntityWorldReactor for StartingReactor
{
    type StartingTriggers = BroadcastTrigger<()>;
    type Triggers = EntityEventTrigger<usize>;
    type Data = ();

    fn reactor(self) -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            move ||
            {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        )
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Reactor with starting and normal triggers.
struct FullReactor(Arc<AtomicU32>);

impl EntityWorldReactor for FullReactor
{
    type StartingTriggers = BroadcastTrigger<()>;
    type Triggers = EntityEventTrigger<usize>;
    type Data = ();

    fn reactor(self) -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            move ||
            {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        )
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

//FullDataReactor

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

// register world reactor, run it manually
#[test]
fn entity_world_reactor_runs_manually()
{
    // setup
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .add_entity_reactor(EmptyReactor(count_inner));
    let world = &mut app.world;

    // run the reactor
    world.syscall((),
        move |mut commands: Commands, reactor: EntityReactor<EmptyReactor>|
        {
            reactor.run(&mut commands);
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 1);
}

//-------------------------------------------------------------------------------------------------------------------

// register world reactor with starting triggers, run it manually
#[test]
fn entity_world_reactor_with_starting_triggers_runs_manually()
{
    // setup
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .add_entity_reactor_with(StartingReactor(count_inner), broadcast::<()>());
    let world = &mut app.world;

    // run the reactor
    world.syscall((),
        move |mut commands: Commands, reactor: EntityReactor<StartingReactor>|
        {
            reactor.run(&mut commands);
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 1);
}

//-------------------------------------------------------------------------------------------------------------------

// register world reactor with starting triggers, triggers fire
#[test]
fn entity_world_reactor_with_starting_triggers_fires()
{
    // setup
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .add_entity_reactor_with(StartingReactor(count_inner), broadcast::<()>());
    let world = &mut app.world;

    // run the reactor
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().broadcast(());
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 1);
}

//-------------------------------------------------------------------------------------------------------------------

// register world reactor with starting triggers, triggers fire, remove triggers, run it manually
#[test]
fn entity_world_reactor_with_starting_triggers_fires_with_removal()
{
    // setup
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .add_entity_reactor_with(StartingReactor(count_inner), broadcast::<()>());
    let world = &mut app.world;

    // trigger the reactor
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().broadcast(());
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // remove triggers
    world.syscall((),
        move |mut c: Commands, mut reactor: EntityReactor<StartingReactor>|
        {
            reactor.remove(&mut c, broadcast::<()>());
        }
    );

    // system should not have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // trigger the reactor
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().broadcast(());
        }
    );

    // system should not have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // run it manually
    world.syscall((),
        move |mut commands: Commands, reactor: EntityReactor<StartingReactor>|
        {
            reactor.run(&mut commands);
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 2);
}

//-------------------------------------------------------------------------------------------------------------------

// register world reactor with starting triggers, add triggers, triggers fire
#[test]
fn entity_world_reactor_with_all_triggers_fires()
{
    // setup
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .add_entity_reactor_with(FullReactor(count_inner), broadcast::<()>());
    let world = &mut app.world;

    // trigger the reactor with starting trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().broadcast(());
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // add trigger
    let entity = world.spawn_empty().id();
    world.syscall((),
        move |mut c: Commands, reactor: EntityReactor<FullReactor>|
        {
            reactor.add(&mut c, entity, ());
        }
    );

    // system should not have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // trigger the reactor with new trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, 0usize);
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 2);

    // trigger the reactor with starting trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().broadcast(());
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 3);
}

//-------------------------------------------------------------------------------------------------------------------

// register world reactor, add triggers, triggers fire, remove triggers, run it manually
#[test]
fn entity_world_reactor_with_all_triggers_fire_and_remove()
{
    // setup
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .add_entity_reactor_with(FullReactor(count_inner), broadcast::<()>());
    let world = &mut app.world;

    // trigger the reactor with starting trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().broadcast(());
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // add trigger
    let entity = world.spawn_empty().id();
    world.syscall((),
        move |mut c: Commands, reactor: EntityReactor<FullReactor>|
        {
            reactor.add(&mut c, entity, ());
        }
    );

    // system should not have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // trigger the reactor with new trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, 0usize);
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 2);

    // trigger the reactor with starting trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().broadcast(());
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 3);

    // remove triggers
    world.syscall((),
        move |mut c: Commands, mut reactor: EntityReactor<FullReactor>|
        {
            reactor.remove(&mut c, (broadcast::<()>(), entity_event::<usize>(entity)));
        }
    );

    // trigger the reactor with old triggers
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().broadcast(());
            c.react().entity_event(entity, 0usize);
        }
    );

    // system should not have run
    assert_eq!(count.load(Ordering::Relaxed), 3);

    // run it manually
    world.syscall((),
        move |mut commands: Commands, reactor: EntityReactor<FullReactor>|
        {
            reactor.run(&mut commands);
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 4);
}

//-------------------------------------------------------------------------------------------------------------------

// reactor without entity reactors, no data found

// reactor with entity reactors, all data found on starting trigger firing

// reactor with entity reactors, only targeted data found on entity trigger firing

//-------------------------------------------------------------------------------------------------------------------
