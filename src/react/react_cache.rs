//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use crossbeam::channel::{Receiver, Sender};

//standard shortcuts
use core::any::TypeId;
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

struct ComponentReactors
{
    insertion_callbacks : Vec<ReactorHandle>,
    mutation_callbacks  : Vec<ReactorHandle>,
    removal_callbacks   : Vec<ReactorHandle>,
}

impl ComponentReactors
{
    fn is_empty(&self) -> bool
    {
        self.insertion_callbacks.is_empty() &&
        self.mutation_callbacks.is_empty()  &&
        self.removal_callbacks.is_empty()  
    }
}

impl Default for ComponentReactors
{
    fn default() -> Self
    {
        Self{
            insertion_callbacks : Vec::new(),
            mutation_callbacks  : Vec::new(),
            removal_callbacks   : Vec::new(),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Collect component removals.
///
/// Note: `RemovedComponents` acts like an event reader, so multiple invocations of this system within one tick will
/// not see duplicate removals.
fn collect_component_removals<C: ReactComponent>(
    In(mut buffer) : In<Vec<Entity>>,
    mut removed    : RemovedComponents<React<C>>,
) -> Vec<Entity>
{
    buffer.clear();
    removed.read().for_each(|entity| buffer.push(entity));
    buffer
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

struct RemovalChecker
{
    component_id : TypeId,
    checker      : SysCall<(), Vec<Entity>, Vec<Entity>>
}

impl RemovalChecker
{
    fn new<C: ReactComponent>() -> Self
    {
        Self{
            component_id : TypeId::of::<C>(),
            checker      : SysCall::new(|world, buffer| syscall(world, buffer, collect_component_removals::<C>)),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Schedules reactions to an entity mutation.
fn schedule_entity_reaction_impl(
    queue           : &mut CobwebCommandQueue<ReactionCommand>,
    reaction_source : Entity,
    reaction_type   : EntityReactionType,
    entity_reactors : &EntityReactors
){
    if let EntityReactionType::Event(id) = reaction_type
    { tracing::error!(?id, "tried queuing entity event as entity reaction"); return; }

    for reactor in entity_reactors.iter_rtype(reaction_type)
    {
        queue.push(
                ReactionCommand::EntityReaction{
                    reaction_source,
                    reaction_type,
                    reactor,
                }
            );
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource)]
pub(crate) struct ReactCache
{
    /// flag that records whether a reaction tree is currently running
    in_reaction_tree: bool,

    /// query to get read-access to entity reactors
    entity_reactors_query: Option<QueryState<&'static EntityReactors>>,

    /// Per-component reactors
    component_reactors: HashMap<TypeId, ComponentReactors>,

    /// Components with removal reactors (cached to prevent duplicate insertion)
    tracked_removals: HashSet<TypeId>,
    /// Component removal checkers (as a vec for efficient iteration)
    removal_checkers: Vec<RemovalChecker>,
    /// Removal checker buffer (cached for reuse)
    removal_buffer: Option<Vec<Entity>>,

    // Entity despawn reactors
    despawn_reactors: HashMap<Entity, Vec<ReactorHandle>>,
    /// Despawn sender (cached for reuse with new despawn trackers)
    despawn_sender: Sender<Entity>,
    /// Despawn receiver
    despawn_receiver: Receiver<Entity>,

    /// Any entity event reactors
    any_entity_event_reactors: HashMap<TypeId, Vec<ReactorHandle>>,

    /// Resource mutation reactors
    resource_reactors: HashMap<TypeId, Vec<ReactorHandle>>,

    /// Broadcast event reactors
    broadcast_reactors: HashMap<TypeId, Vec<ReactorHandle>>,
}

impl ReactCache
{
    /// Starts a reaction tree.
    /// 
    /// Returns `false` if we are already in a reaction tree.
    pub(crate) fn start_reaction_tree(&mut self) -> bool
    {
        if self.in_reaction_tree { return false; }
        self.in_reaction_tree = true;
        true
    }

    /// Ends a reaction tree.
    pub(crate) fn end_reaction_tree(&mut self)
    {
        self.in_reaction_tree = false;
    }

    pub(crate) fn despawn_sender(&self) -> Sender<Entity>
    {
        self.despawn_sender.clone()
    }

    pub(crate) fn track_removals<C: ReactComponent>(&mut self)
    {
        // track removals of this component if untracked
        if self.tracked_removals.contains(&TypeId::of::<C>()) { return; };
        self.tracked_removals.insert(TypeId::of::<C>());
        self.removal_checkers.push(RemovalChecker::new::<C>());
    }

    pub(crate) fn register_insertion_reactor<C: ReactComponent>(&mut self, handle: ReactorHandle)
    {
        self.component_reactors
            .entry(TypeId::of::<C>())
            .or_default()
            .insertion_callbacks
            .push(handle);
    }

    pub(crate) fn register_mutation_reactor<C: ReactComponent>(&mut self, handle: ReactorHandle)
    {
        self.component_reactors
            .entry(TypeId::of::<C>())
            .or_default()
            .mutation_callbacks
            .push(handle);
    }

    pub(crate) fn register_removal_reactor<C: ReactComponent>(&mut self, handle: ReactorHandle)
    {
        self.component_reactors
            .entry(TypeId::of::<C>())
            .or_default()
            .removal_callbacks
            .push(handle);
    }

    pub(crate) fn register_any_entity_event_reactor<E: 'static>(&mut self, handle: ReactorHandle)
    {
        self.any_entity_event_reactors
            .entry(TypeId::of::<E>())
            .or_default()
            .push(handle);
    }

    pub(crate) fn register_resource_mutation_reactor<R: ReactResource>(&mut self, handle: ReactorHandle)
    {
        self.resource_reactors
            .entry(TypeId::of::<R>())
            .or_default()
            .push(handle);
    }

    pub(crate) fn register_broadcast_reactor<E: 'static>(&mut self, handle: ReactorHandle)
    {
        self.broadcast_reactors
            .entry(TypeId::of::<E>())
            .or_default()
            .push(handle);
    }

    pub(crate) fn register_despawn_reactor(&mut self, entity: Entity, handle: ReactorHandle)
    {
        self.despawn_reactors
            .entry(entity)
            .or_default()
            .push(handle);
    }

    /// Revokes a component insertion reactor.
    pub(crate) fn revoke_component_reactor(&mut self, rtype: EntityReactionType, reactor_id: SystemCommand)
    {
        // get cached callbacks
        let (comp_id, reactors) = match rtype
        {
            EntityReactionType::Insertion(comp_id) => (comp_id, self.component_reactors.get_mut(&comp_id)),
            EntityReactionType::Mutation(comp_id)  => (comp_id, self.component_reactors.get_mut(&comp_id)),
            EntityReactionType::Removal(comp_id)   => (comp_id, self.component_reactors.get_mut(&comp_id)),
            EntityReactionType::Event(_)           => unreachable!(),
        };
        let Some(reactors) = reactors else { return; };
        let callbacks = match rtype
        {
            EntityReactionType::Insertion(_) => &mut reactors.insertion_callbacks,
            EntityReactionType::Mutation(_)  => &mut reactors.mutation_callbacks,
            EntityReactionType::Removal(_)   => &mut reactors.removal_callbacks,
            EntityReactionType::Event(_)     => unreachable!(),
        };

        // revoke reactor
        for (idx, handle) in callbacks.iter().enumerate()
        {
            if handle.sys_command() != reactor_id { continue; }
            let _ = callbacks.remove(idx);

            break;
        }

        // cleanup empty hashmap entries
        if !reactors.is_empty() { return; }
        let _ = self.component_reactors.remove(&comp_id);
    }

    /// Revokes a resource mutation reactor.
    pub(crate) fn revoke_any_entity_event_reactor(&mut self, event_id: TypeId, reactor_id: SystemCommand)
    {
        // get callbacks
        let Some(callbacks) = self.any_entity_event_reactors.get_mut(&event_id) else { return; };

        // revoke reactor
        for (idx, handle) in callbacks.iter().enumerate()
        {
            if handle.sys_command() != reactor_id { continue; }
            let _ = callbacks.remove(idx);
            break;
        }

        // cleanup empty hashmap entries
        if callbacks.len() > 0 { return; }
        let _ = self.any_entity_event_reactors.remove(&event_id);
    }

    /// Revokes a resource mutation reactor.
    pub(crate) fn revoke_resource_mutation_reactor(&mut self, resource_id: TypeId, reactor_id: SystemCommand)
    {
        // get callbacks
        let Some(callbacks) = self.resource_reactors.get_mut(&resource_id) else { return; };

        // revoke reactor
        for (idx, handle) in callbacks.iter().enumerate()
        {
            if handle.sys_command() != reactor_id { continue; }
            let _ = callbacks.remove(idx);
            break;
        }

        // cleanup empty hashmap entries
        if callbacks.len() > 0 { return; }
        let _ = self.resource_reactors.remove(&resource_id);
    }

    /// Revokes an event reactor.
    pub(crate) fn revoke_broadcast_reactor(&mut self, event_id: TypeId, reactor_id: SystemCommand)
    {
        // get callbacks
        let Some(callbacks) = self.broadcast_reactors.get_mut(&event_id) else { return; };

        // revoke reactor
        for (idx, handle) in callbacks.iter().enumerate()
        {
            if handle.sys_command() != reactor_id { continue; }
            let _ = callbacks.remove(idx);
            break;
        }

        // cleanup empty hashmap entries
        if callbacks.len() > 0 { return; }
        let _ = self.broadcast_reactors.remove(&event_id);
    }

    /// Revokes a despawn reactor.
    pub(crate) fn revoke_despawn_reactor(&mut self, entity: Entity, reactor_id: SystemCommand)
    {
        // get callbacks
        let Some(callbacks) = self.despawn_reactors.get_mut(&entity) else { return; };

        // revoke reactor
        for (idx, handle) in callbacks.iter().enumerate()
        {
            if handle.sys_command() != reactor_id { continue; }
            let _ = callbacks.remove(idx);
            break;
        }

        // cleanup empty hashmap entries
        if callbacks.len() > 0 { return; }
        let _ = self.despawn_reactors.remove(&entity);
    }

    /// Queues reactions to a component insertion on an entity.
    pub(crate) fn schedule_insertion_reaction<C: ReactComponent>(
        In(entity)      : In<Entity>,
        cache           : Res<ReactCache>,
        mut commands    : Commands,
        mut queue       : ResMut<CobwebCommandQueue<ReactionCommand>>,
        entity_reactors : Query<&EntityReactors>,
    ){
        let rtype = EntityReactionType::Insertion(TypeId::of::<C>());

        // entity-specific reactors
        if let Ok(entity_reactors) = entity_reactors.get(entity)
        {
            let _ = schedule_entity_reaction_impl(&mut queue, entity, rtype, &entity_reactors);
        }

        // entity-agnostic component reactors
        if let Some(handlers) = cache.component_reactors.get(&TypeId::of::<C>())
        {
            for handle in handlers.insertion_callbacks.iter()
            {
                queue.push(
                        ReactionCommand::EntityReaction{
                            reaction_source : entity,
                            reaction_type   : rtype,
                            reactor         : handle.sys_command(),
                        }
                    );
            }
        }

        // reaction tree
        commands.add(reaction_tree);
    }

    /// Queues reactions to a component mutation on an entity.
    pub(crate) fn schedule_mutation_reaction<C: ReactComponent>(
        In(entity)      : In<Entity>,
        cache           : Res<ReactCache>,
        mut commands    : Commands,
        mut queue       : ResMut<CobwebCommandQueue<ReactionCommand>>,
        entity_reactors : Query<&EntityReactors>,
    ){
        let rtype = EntityReactionType::Mutation(TypeId::of::<C>());

        // entity-specific reactors
        if let Ok(entity_reactors) = entity_reactors.get(entity)
        {
            let _ = schedule_entity_reaction_impl(&mut queue, entity, rtype, &entity_reactors);
        }

        // entity-agnostic component reactors
        if let Some(handlers) = cache.component_reactors.get(&TypeId::of::<C>())
        {
            for handle in handlers.mutation_callbacks.iter()
            {
                queue.push(
                        ReactionCommand::EntityReaction{
                            reaction_source : entity,
                            reaction_type   : rtype,
                            reactor         : handle.sys_command(),
                        }
                    );
            }
        }

        // reaction tree
        commands.add(reaction_tree);
    }

    /// Schedules component removal reactors.
    pub(crate) fn schedule_removal_reactions(&mut self, world: &mut World)
    {
        // extract cached
        let mut buffer = self.removal_buffer.take().unwrap_or_else(|| Vec::default());
        let mut query  = self.entity_reactors_query.take().unwrap_or_else(|| world.query::<&EntityReactors>());
        let mut queue  = world.remove_resource::<CobwebCommandQueue<ReactionCommand>>().unwrap();

        // process all removal checkers
        for checker in &mut self.removal_checkers
        {
            // check for removals
            buffer = checker.checker.call(world, buffer);
            if buffer.len() == 0 { continue; }

            // queue removal callbacks
            let rtype = EntityReactionType::Removal(checker.component_id);
            for entity in buffer.iter()
            {
                // entity-specific component reactors
                if let Ok(entity_reactors) = query.get(world, *entity)
                {
                    schedule_entity_reaction_impl(
                            &mut queue,
                            *entity,
                            rtype,
                            &entity_reactors
                        );
                }

                // entity-agnostic component reactors
                let Some(reactors) = self.component_reactors.get(&checker.component_id) else { continue; };
                for handle in reactors.removal_callbacks.iter()
                {
                    queue.push(
                            ReactionCommand::EntityReaction{
                                reaction_source : *entity,
                                reaction_type   : rtype,
                                reactor         : handle.sys_command(),
                            }
                        );
                }
            }
        }

        // return cached
        self.removal_buffer = Some(buffer);
        self.entity_reactors_query = Some(query);
        world.insert_resource(queue);

        // note: `reaction_tree` is not scheduled here because removals/despawns are handled separately
    }

    /// Queues reactions to an entity event.
    pub(crate) fn schedule_entity_event_reaction<E: Send + Sync + 'static>(
        In((target, event)) : In<(Entity, E)>,
        mut commands        : Commands,
        cache               : Res<ReactCache>,
        mut queue           : ResMut<CobwebCommandQueue<ReactionCommand>>,
        entity_reactors     : Query<&EntityReactors>,
    ){
        // get reactors
        let entity_reactors = entity_reactors.get(target);
        let handlers = cache.any_entity_event_reactors.get(&TypeId::of::<E>());

        // if there are no handlers, just drop the event data
        let reaction_type = EntityReactionType::Event(TypeId::of::<E>());
        let num = entity_reactors.map(|e| e.count(reaction_type)).unwrap_or_default()
            + handlers.map(|h| h.len()).unwrap_or_default();
        if num == 0 { return; }

        // prep entity data
        let data_entity = commands.spawn(EntityEventData::new(target, event)).id();

        // entity-specific reactors
        let mut count = 0;
        if let Ok(entity_reactors) = entity_reactors
        {
            for reactor in entity_reactors.iter_rtype(reaction_type)
            {
                count += 1;
                queue.push(
                        ReactionCommand::EntityEvent{
                            target,
                            data_entity,
                            reactor,
                            last_reader: count == num,
                        }
                    );
            }            
        }

        // Entity-agnostic reactors
        if let Some(handlers) = cache.any_entity_event_reactors.get(&TypeId::of::<E>())
        {
            // queue reactors
            for handle in handlers.iter()
            {
                count += 1;
                queue.push(
                    ReactionCommand::EntityEvent{
                        target,
                        data_entity,
                        reactor: handle.sys_command(),
                        last_reader: count == num,
                    }
                );
            }
        }

        // reaction tree
        commands.add(reaction_tree);
    }

    /// Queues reactions to tracked despawns.
    pub(crate) fn schedule_despawn_reactions(&mut self, world: &mut World)
    {
        let mut queue = world.resource_mut::<CobwebCommandQueue<ReactionCommand>>();
        while let Ok(despawned_entity) = self.despawn_receiver.try_recv()
        {
            let Some(mut despawn_reactors) = self.despawn_reactors.remove(&despawned_entity) else { continue; };

            // queue despawn callbacks
            for handle in despawn_reactors.drain(..)
            {
                queue.push(
                        ReactionCommand::Despawn{
                            reaction_source : despawned_entity,
                            reactor         : handle.sys_command(),
                            handle,
                        }
                    );
            }
        }

        // note: `reaction_tree` is not scheduled here because removals/despawns are handled separately
    }

    /// Queues reactions to a resource mutation.
    pub(crate) fn schedule_resource_mutation_reaction<R: ReactResource>(
        cache        : Res<ReactCache>,
        mut commands : Commands,
        mut queue    : ResMut<CobwebCommandQueue<ReactionCommand>>,
    ){
        let Some(handlers) = cache.resource_reactors.get(&TypeId::of::<R>()) else { return; };

        // queue reactors
        for handle in handlers.iter()
        {
            queue.push(
                ReactionCommand::Resource{
                    reactor: handle.sys_command(),
                }
            );
        }

        // reaction tree
        commands.add(reaction_tree);
    }

    /// Queues reactions to a broadcasted event.
    pub(crate) fn schedule_broadcast_reaction<E: Send + Sync + 'static>(
        In(event)    : In<E>,
        cache        : Res<ReactCache>,
        mut commands : Commands,
        mut queue    : ResMut<CobwebCommandQueue<ReactionCommand>>,
    ){
        let Some(handlers) = cache.broadcast_reactors.get(&TypeId::of::<E>()) else { return; };

        // if there are no handlers, just drop the event data
        let num = handlers.len();
        if num == 0 { return; }

        // prep event data
        let data_entity = commands.spawn(BroadcastEventData::new(event)).id();

        // queue reactors
        for (idx, handle) in handlers.iter().enumerate()
        {
            queue.push(
                ReactionCommand::BroadcastEvent{
                    data_entity,
                    reactor     : handle.sys_command(),
                    last_reader : idx + 1 == num,
                }
            );
        }

        // reaction tree
        commands.add(reaction_tree);
    }
}

impl Default for ReactCache
{
    fn default() -> Self
    {
        // prep despawn channel
        let (despawn_sender, despawn_receiver) = crossbeam::channel::unbounded();

        Self{
            in_reaction_tree      : false,
            entity_reactors_query : None,
            component_reactors    : HashMap::default(),
            tracked_removals      : HashSet::default(),
            removal_checkers      : Vec::new(),
            removal_buffer        : None,
            despawn_reactors      : HashMap::new(),
            despawn_sender,
            despawn_receiver,
            any_entity_event_reactors : HashMap::new(),
            resource_reactors         : HashMap::new(),
            broadcast_reactors        : HashMap::new(),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
