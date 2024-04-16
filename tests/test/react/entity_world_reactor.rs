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

/// Reactor with starting and normal triggers and entity data. Detects when data is read.
struct FullDataReactorDetector(Arc<AtomicU32>);

impl EntityWorldReactor for FullDataReactorDetector
{
    type Triggers = EntityEventTrigger<()>;
    type Data = ();

    fn reactor(self) -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            move |data: ReactorData<Self>|
            {
                for _ in data.iter()
                {
                    self.0.fetch_add(1, Ordering::Relaxed);
                }
            }
        )
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------
/*
/// Reactor with entity data that is mutated.
struct FullDataReactorMutable(Arc<AtomicU32>);

impl EntityWorldReactor for FullDataReactorMutable
{
    type StartingTriggers = ();
    type Triggers = EntityEventTrigger<usize>;
    type Data = usize;

    fn reactor(self) -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            move |mut data: ReactorData<Self>, event: EntityEvent<usize>|
            {
                let (event_entity, event_data) = event.read().unwrap();

                assert_eq!(data.iter().count(), 1);
                for (entity, entity_data) in data.iter_mut()
                {
                    assert_eq!(event_entity, entity);
                    let new_data = *event_data + *entity_data;
                    self.0.store(new_data as u32, Ordering::Relaxed);
                    *entity_data += new_data as usize;
                }
            }
        )
    }
}
 */
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

// register world reactor, add trigger, trigger fires
#[test]
fn entity_world_reactor_basic()
{
    // setup
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .add_entity_reactor(FullReactor(count_inner));
    let world = &mut app.world;

    // add trigger
    let entity = world.spawn_empty().id();
    world.syscall((),
        move |mut c: Commands, reactor: EntityReactor<FullReactor>|
        {
            reactor.add(&mut c, entity, ());
        }
    );

    // system should not have run
    assert_eq!(count.load(Ordering::Relaxed), 0);

    // trigger the reactor with new trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, 0usize);
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 1);
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
        .add_entity_reactor(FullReactor(count_inner));
    let world = &mut app.world;

    // add trigger
    let entity = world.spawn_empty().id();
    world.syscall((),
        move |mut c: Commands, reactor: EntityReactor<FullReactor>|
        {
            reactor.add(&mut c, entity, ());
        }
    );

    // system should not have run
    assert_eq!(count.load(Ordering::Relaxed), 0);

    // trigger the reactor with new trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, 0usize);
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // remove trigger
    world.syscall((),
        move |mut c: Commands, reactor: EntityReactor<FullReactor>|
        {
            reactor.remove(&mut c, entity_event::<usize>(entity));
        }
    );

    // trigger the reactor with old trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, 0usize);
        }
    );

    // system should not have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // run it manually
    world.syscall((),
        move |mut commands: Commands, reactor: EntityReactor<FullReactor>|
        {
            reactor.run(&mut commands);
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 2);
}

//-------------------------------------------------------------------------------------------------------------------

// reactor sees data appropriately depending on registered entities
#[test]
fn entity_world_reactor_data_checks()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    */

    // setup
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .add_entity_reactor(FullDataReactorDetector(count_inner));
    let world = &mut app.world;

    // add trigger
    let entity = world.spawn_empty().id();
    world.syscall((),
        move |mut c: Commands, reactor: EntityReactor<FullDataReactorDetector>|
        {
            reactor.add(&mut c, entity, ());
        }
    );

    // system should not have run/seen data
    assert_eq!(count.load(Ordering::Relaxed), 0);

    // trigger the reactor with new trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, ());
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // add another trigger
    let entity2 = world.spawn_empty().id();
    world.syscall((),
        move |mut c: Commands, reactor: EntityReactor<FullDataReactorDetector>|
        {
            reactor.add(&mut c, entity2, ());
        }
    );

    // trigger the reactor with original trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, ());
        }
    );

    // system should have seen one data
    assert_eq!(count.load(Ordering::Relaxed), 2);

    // trigger the reactor with second trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity2, ());
        }
    );

    // system should have seen one data
    assert_eq!(count.load(Ordering::Relaxed), 3);
}

//-------------------------------------------------------------------------------------------------------------------
/*
// reactor with data should be mutable
#[test]
fn entity_world_reactor_mutable_data()
{
    // setup
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .add_entity_reactor(FullDataReactorMutable(count_inner));
    let world = &mut app.world;

    // add trigger
    let entity = world.spawn_empty().id();
    world.syscall((),
        move |mut c: Commands, reactor: EntityReactor<FullDataReactorMutable>|
        {
            reactor.add(&mut c, entity, 0usize);
        }
    );

    // system should not have run/seen data
    assert_eq!(count.load(Ordering::Relaxed), 0);

    // trigger the reactor with new trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, 1);
        }
    );

    // system should have run
    assert_eq!(count.load(Ordering::Relaxed), 1);

    // trigger the reactor again
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, 1);
        }
    );

    // system should have run and seen the data
    assert_eq!(count.load(Ordering::Relaxed), 2);

    // add another trigger
    let entity2 = world.spawn_empty().id();
    world.syscall((),
        move |mut c: Commands, reactor: EntityReactor<FullDataReactorMutable>|
        {
            reactor.add(&mut c, entity2, 0usize);
        }
    );

    // trigger the reactor with original trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, 2usize);
        }
    );

    // system should have seen one data
    assert_eq!(count.load(Ordering::Relaxed), 4);

    // trigger the reactor with second trigger
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().entity_event(entity, 3usize);
        }
    );

    // system should have run and seen both entities' data
    assert_eq!(count.load(Ordering::Relaxed), 7);
}
 */
//-------------------------------------------------------------------------------------------------------------------
