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

pub fn update_test_recorder_with_broadcast(mut event: BroadcastEvent<IntEvent>, mut recorder: ResMut<TestReactRecorder>)
{
    let Some(event) = event.read() else { return; };
    recorder.0 = event.0;
}

//-------------------------------------------------------------------------------------------------------------------

pub fn update_test_recorder_with_broadcast_and_recurse(
    mut rcommands : ReactCommands,
    mut event     : BroadcastEvent<IntEvent>,
    mut recorder  : ResMut<TestReactRecorder>
){
    let Some(event) = event.read() else { return; };
    recorder.0 += 1;

    // recurse until the event is 0
    if event.0 == 0 { return; }
    rcommands.broadcast(IntEvent(event.0.saturating_sub(1)));
}

//-------------------------------------------------------------------------------------------------------------------

pub fn update_test_recorder_with_broadcast_and_resource(
    mut event    : BroadcastEvent<IntEvent>,
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

pub fn on_entity_insertion(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(entity_insertion::<TestComponent>(entity),
            move |world: &mut World| syscall(world, entity, update_test_recorder_with_component)
        )
}

pub fn on_entity_mutation(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(entity_mutation::<TestComponent>(entity),
            move |world: &mut World| syscall(world, entity, update_test_recorder_with_component)
        )
}

pub fn on_entity_removal(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(entity_removal::<TestComponent>(entity), infinitize_test_recorder)
}

pub fn on_insertion(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(insertion::<TestComponent>(), update_test_recorder_on_insertion)
}

pub fn on_mutation(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(mutation::<TestComponent>(), update_test_recorder_on_mutation)
}

pub fn on_removal(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(removal::<TestComponent>(), |_, world: &mut World| syscall(world, (), infinitize_test_recorder))
}

pub fn on_despawn(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(despawn(entity), infinitize_test_recorder)
}

pub fn on_despawn_div2(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(despawn(entity), test_recorder_div2)
}

pub fn on_resource_mutation(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(resource_mutation::<TestReactRes>(), update_test_recorder_with_resource)
}

pub fn on_resource_mutation_once(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.once(resource_mutation::<TestReactRes>(), update_test_recorder_with_resource)
}

pub fn on_broadcast(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(broadcast::<IntEvent>(), update_test_recorder_with_broadcast)
}

pub fn on_broadcast_recursive(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(broadcast::<IntEvent>(), update_test_recorder_with_broadcast_and_recurse)
}

pub fn on_broadcast_or_resource(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on((broadcast::<IntEvent>(), resource_mutation::<TestReactRes>()),
        update_test_recorder_with_broadcast_and_resource)
}

//-------------------------------------------------------------------------------------------------------------------

pub fn insert_on_test_entity(In((entity, component)): In<(Entity, TestComponent)>, mut rcommands: ReactCommands)
{
    rcommands.insert(entity, component);
}

//-------------------------------------------------------------------------------------------------------------------

pub fn remove_from_test_entity(In(entity): In<Entity>, mut commands: Commands)
{
    commands.get_entity(entity).unwrap().remove::<React<TestComponent>>();
}

//-------------------------------------------------------------------------------------------------------------------

pub fn update_test_entity(
    In((entity, new_val)) : In<(Entity, TestComponent)>,
    mut rcommands         : ReactCommands,
    mut test_entities     : Query<&mut React<TestComponent>>,
){
    *test_entities
        .get_mut(entity)
        .unwrap()
        .get_mut(&mut rcommands) = new_val;
}

//-------------------------------------------------------------------------------------------------------------------

pub fn update_react_res(
    In(new_val)   : In<usize>,
    mut rcommands : ReactCommands,
    mut react_res : ReactResMut<TestReactRes>
){
    react_res.get_mut(&mut rcommands).0 = new_val;
}

//-------------------------------------------------------------------------------------------------------------------

pub fn send_broadcast(In(data): In<usize>, mut rcommands: ReactCommands)
{
    rcommands.broadcast(IntEvent(data));
}

//-------------------------------------------------------------------------------------------------------------------

pub fn on_entity_mutation_chain_to_res(In(entity): In<Entity>, mut rcommands: ReactCommands)
{
    rcommands.on(entity_mutation::<TestComponent>(entity),
            move
            |
                mut rcommands : ReactCommands,
                mut react_res : ReactResMut<TestReactRes>,
                test_entities : Query<&React<TestComponent>>
            |
            {
                react_res.get_mut(&mut rcommands).0 = test_entities.get(entity).unwrap().0;
            }
        );
}

//-------------------------------------------------------------------------------------------------------------------

pub fn revoke_reactor(In(token): In<RevokeToken>, mut rcommands: ReactCommands)
{
    rcommands.revoke(token);
}

//-------------------------------------------------------------------------------------------------------------------
