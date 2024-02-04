//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::ecs::system::CommandQueue;
use bevy::prelude::*;

//standard shortcuts
use core::ops::Deref;
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Cached command queues for react methods.
/// - We use a container of command queues in case of recursion.
#[derive(Resource, Default)]
struct ReactCommandQueue(Vec<CommandQueue>);

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Get a command queue from the react command queue cache.
fn pop_react_command_queue(world: &mut World) -> CommandQueue
{
    world.get_resource_or_insert_with(|| ReactCommandQueue::default())
        .0
        .pop()
        .unwrap_or_else(|| CommandQueue::default())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Put command queue back in react command queue cache.
fn push_react_command_queue(world: &mut World, queue: CommandQueue)
{
    world.get_resource_or_insert_with(|| ReactCommandQueue::default()).0.push(queue);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// React to tracked despawns.
/// - Returns number of callbacks queued.
//note: we cannot use RemovedComponents here because we need the ability to react to despawns that occur between
//      when 'register despawn tracker' is queued and executed
fn react_to_despawns_impl(
    mut commands    : Commands,
    mut react_cache : ResMut<ReactCache>,
) -> usize
{
    let mut callback_count = 0;

    while let Some(despawned_entity) = react_cache.try_recv_despawn()
    {
        // remove prepared callbacks
        let Some(mut despawn_callbacks) = react_cache.remove_despawn_reactors(despawned_entity) else { continue; };

        // queue despawn callbacks
        for (_, despawn_callback) in despawn_callbacks.drain(..)
        {
            enque_command(&mut commands, despawn_callback);
            callback_count += 1;
        }
    }

    callback_count
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Tag trait for reactive components.
///
/// It is not recommended to add `ReactComponent` and `Component` to the same struct, as it will likely cause confusion.
pub trait ReactComponent: Send + Sync + 'static {}

//-------------------------------------------------------------------------------------------------------------------

/// Component wrapper that enables reacting to component mutations.
/// - WARNING: It is possible to remove a `React` from one entity and manually insert it to another entity. That WILL
///            break the react framework. Instead use `react_commands.insert(new_entity, react_component.take());`.
#[derive(Component)]
pub struct React<C: ReactComponent>
{
    pub(crate) entity    : Entity,
    pub(crate) component : C,
}

impl<C: ReactComponent> React<C>
{
    /// Mutably access the component and trigger reactions.
    pub fn get_mut<'a>(&'a mut self, rcommands: &mut ReactCommands) -> &'a mut C
    {
        rcommands.cache.react_to_mutation::<C>(&mut rcommands.commands, self.entity);
        &mut self.component
    }

    /// Mutably access the component without triggering reactions.
    pub fn get_mut_noreact(&mut self) -> &mut C
    {
        &mut self.component
    }

    /// Unwrap the `React`.
    pub fn take(self) -> C
    {
        self.component
    }
}

impl<C: ReactComponent> Deref for React<C>
{
    type Target = C;

    fn deref(&self) -> &C
    {
        &self.component
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// React to component removals.
/// - Returns the number of callbacks queued.
pub fn react_to_removals(world: &mut World) -> usize
{
    // remove cached
    let mut react_cache = world.remove_resource::<ReactCache>().expect("ReactCache is missing for removal reactions");
    let mut command_queue = pop_react_command_queue(world);

    // process removals
    let callback_count = react_cache.react_to_removals(world, &mut command_queue);

    // return react cache
    world.insert_resource(react_cache);

    // apply queued reactions
    command_queue.apply(world);

    // return command queue
    push_react_command_queue(world, command_queue);

    callback_count
}

//-------------------------------------------------------------------------------------------------------------------

/// React to tracked despawns.
/// - Returns the number of callbacks queued.
pub fn react_to_despawns(world: &mut World) -> usize
{
    // handle despawns
    syscall(world, (), react_to_despawns_impl)
}

//-------------------------------------------------------------------------------------------------------------------

/// Iteratively react to component removals and entity despawns until all reaction chains have ended.
pub fn react_to_all_removals_and_despawns(world: &mut World)
{
    // loop until no more removals/despawns
    while syscall(world, (), react_to_removals) > 0 || syscall(world, (), react_to_despawns_impl) > 0 {}
}

//-------------------------------------------------------------------------------------------------------------------
