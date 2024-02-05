//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Stores a callable system command.
///
/// We store the callback in an option in order to avoid archetype moves when taking/reinserting the callback in order to
/// call it.
#[derive(Component)]
struct SystemCommand
{
    callback: Option<SystemCommandCallback>,
}

impl SystemCommand
{
    fn new(callback: SystemCommandCallback) -> Self
    {
        Self{ callback: Some(callback) }
    }

    fn take(&mut self) -> Option<SystemCommandCallback>
    {
        self.callback.take()
    }

    fn insert(&mut self, callback: SystemCommandCallback)
    {
        self.callback = Some(callback);
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Used to inject an entity-targeting cleanup function into system commands.
#[derive(Debug, Default, Copy, Clone)]
pub(crate) struct SystemCommandCleanup
{
    cleanup: Option<fn(&mut World, Option<Entity>)>,
    entity: Option<Entity>,
}

impl SystemCommandCleanup
{
    pub(crate) fn new(cleanup: fn(&mut World, Option<Entity>), entity: Option<Entity>) -> Self
    {
        Self{ cleanup: Some(cleanup), entity }
    }

    pub(crate) fn cleanup(self, world: &mut World)
    {
        let Some(cleanup) = self.cleanup else { return; };
        (cleanup)(world, self.entity);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Owns a system command callback.
//todo: wrap the callback in a trait that lets you reassign the injected callback if it is the same type
#[derive(Default, Component)]
pub struct SystemCommandCallback
{
    inner: Box<dyn FnMut(&mut World, SystemCommandCleanup) + Send + Sync 'static>,
}

impl SystemCommandCallback
{
    /// Makes a new system command callback.
    pub fn new(callback: impl FnMut(&mut World, SystemCommandCleanup) + Send + Sync 'static) -> Self
    {
        Self{ inner: Box::new(callback) }
    }

    /// Runs the system command callback.
    ///
    /// The `cleanup` should be invoked between running the command's inner system and
    /// calling `apply_deferred` on the inner system.
    pub fn run(&mut self, world: &mut World, cleanup: SystemCommandCleanup)
    {
        (self.inner)(world, cleanup);
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Copy, Clone, Deref)]
pub(crate) struct SystemCommand(pub(crate) SysId);

impl SystemCommand
{
    pub(crate) fn run(self, &mut World)
    {
        syscommand(world, self.0, SystemCommandCleanup::default());
    }
}

//-------------------------------------------------------------------------------------------------------------------

//todo: validate that data entities will always be cleaned up
#[derive(Debug, Copy, Clone)]
pub(crate) struct EventCommand
{
    pub(crate) system: SysId,
    pub(crate) data_entity: Entity,
}

impl EventCommand
{
    pub(crate) fn run(self, &mut World)
    {
        fn despawn_entity(world: &mut World, entity: Option<Entity>)
        {
            let Some(entity) = entity else { return; };
            world.despawn(entity);
        }
        syscommand(world, self.0, SystemCommandCleanup::new(despawn_entity, Some(self.data_entity)));
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum EntityReactionType
{
    Insertion(TypeId),
    Removal(TypeId),
    Mutation(TypeId),
    Despawn,
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum ReactionCommand
{
    /// A reaction to a resource mutation.
    ResourceReaction
    {
        reactor: SysId,
    },
    /// A reaction to an entity mutation.
    EntityReaction
    {
        reaction_type: EntityReactionType,
        reaction_source: Entity,
        reactor: SysId,
    },
    /// A reaction to a broadcasted event.
    BroadcastEvent
    {
        data_id: TypeId,
        data_entity: Entity,
        reactor: SysId,
    },
    /// A reaction to an event targeted at a specific entity.
    EntityEvent
    {
        data_id: TypeId,
        data_entity: Entity,
        target_entity: Entity,
        reactor: SysId,
    },
}

impl ReactionCommand
{
    pub(crate) fn run(self, &mut World)
    {
        match self
        {
            Self::ResourceReaction{ reactor } =>
            {
                //cleanup: none
            }
            Self::EntityReaction{ reactor } =>
            {
                //update entity reactions resource w/ source entity and reaction type + mark 'on'
                //cleanup: toggle resource 'off'
            }
            Self::BroadcastEvent{ reactor } =>
            {
                //update broadcast event resource w/ data entity and event type id + mark 'on'
                //cleanup: toggle resource 'off'
            }
            Self::EntityEvent{ reactor } =>
            {
                //update entity event resource w/ data entity and event type id and target entity + mark 'on'
                //cleanup: toggle resource 'off'
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Buffers queued cobweb commands of type `T`.
#[derive(Resource, Default)]
pub(crate) struct CobwebCommandQueue<T: Send + Sync + 'static>
{
    /// Queued commands.
    commands: VecDeque<T>,

    /// Cached buffers for storing commands.
    buffers: Vec<VecDeque<T>>,
}

impl<T: Send + Sync + 'static> CobwebCommandQueue<T>
{
    /// Removes all the cobweb commands in a command queue.
    pub(crate) fn remove(&mut self) -> VecDeque<T>
    {
        let replacement = self.buffers.pop().unwrap_or_default();
        std::mem::replace(&mut self.commands, replacement)
    }

    /// Adds a cobweb command.
    pub(crate) fn add(&mut self, command: T)
    {
        self.commands.push(command);
    }

    /// Removes a command from the front of the queue.
    pub(crate) fn pop_front(&mut self) -> Option<T>
    {
        self.commands.pop_front()
    }

    /// Pushes a list of cobweb commands to the end of a command queue.
    pub(crate) fn append(&mut self, mut new: VecDeque<T>)
    {
        if new.len() > 0
        {
            self.commands.append(&mut new);
        }
        self.buffers.push(new);
    }

    /// Pushes a list of cobweb commands to the front of a command queue.
    pub(crate) fn prepend(&mut self, mut new: VecDeque<T>)
    {
        if new.len() == 0
        {
            self.buffers.push(new);
            return;
        }

        let inner = self.commands;
        new.append(inner);
        let buffer = std::mem::replace(inner, new);
        self.buffers.push(buffer);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Executes a system command on the world.
///
/// System commands scheduled by this system command will be run recursively.
pub fn syscommand(world: &mut World, id: SysId, cleanup: SystemCommandCleanup)
{
    // extract the callback
    let Some(mut entity_mut) = world.get_entity_mut(id.entity())
    else
    {
        cleanup(world);
        return;
    };
    let Some(mut system_command) = entity_mut.get_mut::<SystemCommand>()
    else
    {
        tracing::error!(?id, "system command component is missing");
        cleanup(world);
        return;
    };
    let Some(mut callback) = system_command.take()
    else
    {
        tracing::warn!(?id, "system command missing");
        cleanup(world);
        return;
    };

    // remove existing system commands temporarily
    let preexisting_syscommands = world.resource_mut::<CobwebCommandCache>().remove(CobwebCommandType::Command);

    // invoke the callback
    callback.run(world, event_entity);

    // reinsert the callback if its target hasn't been despawned
    // - We don't log an error if the entity is missing in case the callback despawned itself (e.g. one-off commands).
    if let Some(mut entity_mut) = world.get_entity_mut(id.entity())
    {
        if let Some(mut system_command) = entity_mut.get_mut::<SystemCommand>()
        {
            system_command.insert(callback);
        }
        else
        {
            tracing::error!(?id, "system command component is missing");
        }
    }

    // recurse over new system commands
    // - Note that when we recurse, any additional commands from this scope will be removed and reinserted, so this
    //   loop will only act on commands added by the system command for this scope.
    while let Some(next_command) = world.resource_mut::<CobwebCommandCache>().pop_front(CobwebCommandType::Command);
    {
        next_command.run(world);
    }

    // replace previously-existing system commands
    world.resource_mut::<CobwebCommandCache>().append(preexisting_syscommands, CobwebCommandType::Command);

    Ok(())
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) fn reaction_tree(world: &mut World)
{
    // Set the reaction tree flag to prevent the reaction tree from being recursively scheduled.
    world.resource_mut::<ReactCache>().set_reaction_tree();

    let mut reaction_queue = world.resource_mut::<CobwebCommandCache>().remove(CobwebCommandType::Reaction);
    let mut event_queue = world.resource_mut::<CobwebCommandCache>().remove(CobwebCommandType::Event);

    'r: loop
    {
        'e: loop
        {
            // run all system commands recursively
            while let Some(next_command) = world.resource_mut::<CobwebCommandCache>().pop_front(CobwebCommandType::Command);
            {
                next_command.run(world);
            }

            // new events go to the front
            world.resource_mut::<CobwebCommandCache>().append(std::mem::take(event_queue), CobwebCommandType::Event);
            event_queue = world.resource_mut::<CobwebCommandCache>().remove(CobwebCommandType::Event);

            // run one system event
            let Some(next_event) = event_queue.pop_front() else { break 'e; };
            next_event.run(world);
        }

        // new reactions go to the front
        world.resource_mut::<CobwebCommandCache>().append(std::mem::take(reaction_queue), CobwebCommandType::Reaction);
        reaction_queue = world.resource_mut::<CobwebCommandCache>().remove(CobwebCommandType::Reaction);

        // run one reaction
        let Some(next_reaction) = reaction_queue.pop_front() else { break 'r; };
        next_reaction.run(world);
    }

    world.resource_mut::<CobwebCommandCache>().append(event_queue, CobwebCommandType::Event);
    world.resource_mut::<CobwebCommandCache>().append(reaction_queue, CobwebCommandType::Reaction);

    // Unset the reaction tree flag now that we are returning to user-land.
    world.resource_mut::<ReactCache>().unset_reaction_tree();
}

//-------------------------------------------------------------------------------------------------------------------
