//local shortcuts
use bevy_cobweb::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

#[derive(ReactComponent)]
pub struct TestComponent(pub usize);

//-------------------------------------------------------------------------------------------------------------------

#[derive(ReactResource, Default)]
pub struct TestReactRes(pub usize);

//-------------------------------------------------------------------------------------------------------------------

#[derive(Clone)]
pub struct IntEvent(pub usize);

//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct TestReactRecorder(pub usize);

//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Default, Deref, DerefMut)]
pub struct TelescopeHistory(Vec<usize>);

//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Deref, DerefMut)]
pub struct SavedSystemCommand(pub Option<SystemCommand>);

//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Deref, DerefMut)]
pub struct SavedSystemCommands(pub Vec<SystemCommand>);

//-------------------------------------------------------------------------------------------------------------------

pub fn infinitize_test_recorder(mut recorder: ResMut<TestReactRecorder>)
{
    recorder.0 = usize::MAX;
}

//-------------------------------------------------------------------------------------------------------------------

pub fn test_recorder_div2(mut recorder: ResMut<TestReactRecorder>)
{
    recorder.0 /= 2;
}

//-------------------------------------------------------------------------------------------------------------------

/// Copy test component to recorder
pub fn update_test_recorder_with_component(
    In(entity)    : In<Entity>,
    mut recorder  : ResMut<TestReactRecorder>,
    test_entities : Query<&React<TestComponent>>,
){
    recorder.0 = test_entities.get(entity).unwrap().0;
}

//-------------------------------------------------------------------------------------------------------------------

/// Copy test component to recorder
pub fn update_test_recorder_on_insertion(
    entity        : InsertionEvent<TestComponent>,
    mut recorder  : ResMut<TestReactRecorder>,
    test_entities : Query<&React<TestComponent>>,
){
    recorder.0 = test_entities.get(entity.read().unwrap()).unwrap().0;
}

//-------------------------------------------------------------------------------------------------------------------

/// Copy test component to recorder
pub fn update_test_recorder_on_mutation(
    entity        : MutationEvent<TestComponent>,
    mut recorder  : ResMut<TestReactRecorder>,
    test_entities : Query<&React<TestComponent>>,
){
    recorder.0 = test_entities.get(entity.read().unwrap()).unwrap().0;
}

//-------------------------------------------------------------------------------------------------------------------

/// Copy test component to recorder
pub fn update_test_recorder_with_resource(
    mut recorder : ResMut<TestReactRecorder>,
    resource     : ReactRes<TestReactRes>,
){
    recorder.0 = resource.0;
}

//-------------------------------------------------------------------------------------------------------------------

pub fn update_test_recorder_with_broadcast(event: BroadcastEvent<IntEvent>, mut recorder: ResMut<TestReactRecorder>)
{
    let Some(event) = event.read() else { return; };
    recorder.0 = event.0;
}
//-------------------------------------------------------------------------------------------------------------------

pub fn update_test_recorder_with_entity_event(event: EntityEvent<IntEvent>, mut recorder: ResMut<TestReactRecorder>)
{
    let Some((_, event)) = event.read() else { return; };
    recorder.0 = event.0;
}

//-------------------------------------------------------------------------------------------------------------------

pub fn update_test_recorder_with_broadcast_and_recurse(
    mut c : Commands,
    event         : BroadcastEvent<IntEvent>,
    mut recorder  : ResMut<TestReactRecorder>
){
    let Some(event) = event.read() else { return; };
    recorder.0 += 1;

    // recurse until the event is 0
    if event.0 == 0 { return; }
    c.react().broadcast(IntEvent(event.0.saturating_sub(1)));
}

//-------------------------------------------------------------------------------------------------------------------

pub fn update_test_recorder_with_broadcast_and_resource(
    event        : BroadcastEvent<IntEvent>,
    mut recorder : ResMut<TestReactRecorder>,
    resource     : ReactRes<TestReactRes>,
){
    if let Some(event) = event.read()
    {
        recorder.0 += event.0;
    }
    else
    {
        recorder.0 += resource.0;
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub fn insert_on_test_entity(In((entity, component)): In<(Entity, TestComponent)>, mut c: Commands)
{
    c.react().insert(entity, component);
}

//-------------------------------------------------------------------------------------------------------------------

pub fn remove_from_test_entity(In(entity): In<Entity>, mut commands: Commands)
{
    commands.get_entity(entity).unwrap().remove::<React<TestComponent>>();
}

//-------------------------------------------------------------------------------------------------------------------

pub fn update_test_entity(
    In((entity, new_val)) : In<(Entity, TestComponent)>,
    mut c         : Commands,
    mut test_entities     : ReactiveMut<TestComponent>,
){
    *test_entities
        .get_mut(&mut c, entity)
        .unwrap() = new_val;
}

//-------------------------------------------------------------------------------------------------------------------

pub fn update_react_res(
    In(new_val)   : In<usize>,
    mut c         : Commands,
    mut react_res : ReactResMut<TestReactRes>
){
    react_res.get_mut(&mut c).0 = new_val;
}

//-------------------------------------------------------------------------------------------------------------------

pub fn send_broadcast(In(data): In<usize>, mut c: Commands)
{
    c.react().broadcast(IntEvent(data));
}
//-------------------------------------------------------------------------------------------------------------------

pub fn send_entity_event(In((entity, data)): In<(Entity, usize)>, mut c: Commands)
{
    c.react().entity_event(entity, IntEvent(data));
}

//-------------------------------------------------------------------------------------------------------------------

pub fn revoke_reactor(In(token): In<RevokeToken>, mut c: Commands)
{
    c.react().revoke(token);
}

//-------------------------------------------------------------------------------------------------------------------
