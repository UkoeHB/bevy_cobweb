//local shortcuts
use crate::prelude::*;
use bevy_kot_utils::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};

//standard shortcuts
use core::any::TypeId;
use std::vec::Vec;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

struct ComponentReactors
{
    insertion_callbacks : Vec<AutoDespawnSignal>,
    mutation_callbacks  : Vec<AutoDespawnSignal>,
    removal_callbacks   : Vec<AutoDespawnSignal>,
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

/// Schedules a reaction to an entity mutation.
fn schedule_entity_reaction_impl(
    queue           : &mut CobwebCommandQueue<ReactionCommand>,
    reaction_source : Entity,
    reaction_type   : EntityReactionType,
    entity_reactors : &EntityReactors
){
    // get cached callbacks
    let callbacks = match reaction_type
    {
        EntityReactionType::Insertion(id) => entity_reactors.insertion_callbacks.get(&id),
        EntityReactionType::Mutation(id)  => entity_reactors.mutation_callbacks.get(&id),
        EntityReactionType::Removal(id)   => entity_reactors.removal_callbacks.get(&id),
    };
    let Some(callbacks) = callbacks else { return; };

    // queue callbacks
    for sys_handle in callbacks.iter()
    {
        queue.push(
                ReactionCommand::EntityReaction{
                    reaction_source,
                    reaction_type,
                    reactor: SystemCommand(sys_handle.entity()),
                }
            );
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Schedules a reaction to an entity mutation.
fn schedule_entity_reaction(
    In((rtype, entity)) : In<(EntityReactionType, Entity)>,
    mut queue           : ResMut<CobwebCommandQueue<ReactionCommand>>,
    entity_reactors     : Query<&EntityReactors>,
){
    // get this entity's entity reactors
    let Ok(entity_reactors) = entity_reactors.get(entity) else { return; };

    // react
    let _ = schedule_entity_reaction_impl(&mut queue, entity, rtype, &entity_reactors);
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
    despawn_reactors: HashMap<Entity, Vec<AutoDespawnSignal>>,
    /// Despawn sender (cached for reuse with new despawn trackers)
    despawn_sender: Sender<Entity>,
    /// Despawn receiver
    despawn_receiver: Receiver<Entity>,

    /// Resource mutation reactors
    resource_reactors: HashMap<TypeId, Vec<AutoDespawnSignal>>,

    /// Broadcast event reactors
    broadcast_reactors: HashMap<TypeId, Vec<AutoDespawnSignal>>,
    /// Entity event reactors
    entity_event_reactors: HashMap<(Entity, TypeId), Vec<AutoDespawnSignal>>,
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

    pub(crate) fn register_insertion_reactor<C: ReactComponent>(&mut self, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        self.component_reactors
            .entry(TypeId::of::<C>())
            .or_default()
            .insertion_callbacks
            .push(sys_handle.clone());

        ReactorType::ComponentInsertion(TypeId::of::<C>())
    }

    pub(crate) fn register_mutation_reactor<C: ReactComponent>(&mut self, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        self.component_reactors
            .entry(TypeId::of::<C>())
            .or_default()
            .mutation_callbacks
            .push(sys_handle.clone());

        ReactorType::ComponentMutation(TypeId::of::<C>())
    }

    pub(crate) fn register_removal_reactor<C: ReactComponent>(&mut self, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        self.component_reactors
            .entry(TypeId::of::<C>())
            .or_default()
            .removal_callbacks
            .push(sys_handle.clone());

        ReactorType::ComponentRemoval(TypeId::of::<C>())
    }

    pub(crate) fn register_resource_mutation_reactor<R: ReactResource>(
        &mut self,
        sys_handle: &AutoDespawnSignal,
    ) -> ReactorType
    {
        self.resource_reactors
            .entry(TypeId::of::<R>())
            .or_default()
            .push(sys_handle.clone());

        ReactorType::ResourceMutation(TypeId::of::<R>())
    }

    pub(crate) fn register_broadcast_reactor<E: 'static>(&mut self, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        self.broadcast_reactors
            .entry(TypeId::of::<E>())
            .or_default()
            .push(sys_handle.clone());

        ReactorType::Broadcast(TypeId::of::<E>())
    }

    pub(crate) fn register_entity_event_reactor<E: 'static>(
        &mut self,
        target     : Entity,
        sys_handle : &AutoDespawnSignal
    ) -> ReactorType
    {
        self.entity_event_reactors
            .entry((target, TypeId::of::<E>()))
            .or_default()
            .push(sys_handle.clone());

        ReactorType::EntityEvent(target, TypeId::of::<E>())
    }

    pub(crate) fn register_despawn_reactor(&mut self, entity: Entity, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        self.despawn_reactors
            .entry(entity)
            .or_default()
            .push(sys_handle.clone());

        ReactorType::Despawn(entity)
    }

    /// Revokes a component insertion reactor.
    pub(crate) fn revoke_component_reactor(&mut self, rtype: EntityReactionType, reactor_id: u64)
    {
        // get cached callbacks
        let (comp_id, reactors) = match rtype
        {
            EntityReactionType::Insertion(comp_id) => (comp_id, self.component_reactors.get_mut(&comp_id)),
            EntityReactionType::Mutation(comp_id)  => (comp_id, self.component_reactors.get_mut(&comp_id)),
            EntityReactionType::Removal(comp_id)   => (comp_id, self.component_reactors.get_mut(&comp_id)),
        };
        let Some(reactors) = reactors else { return; };
        let callbacks = match rtype
        {
            EntityReactionType::Insertion(_) => &mut reactors.insertion_callbacks,
            EntityReactionType::Mutation(_)  => &mut reactors.mutation_callbacks,
            EntityReactionType::Removal(_)   => &mut reactors.removal_callbacks,
        };

        // revoke reactor
        for (idx, sys_handle) in callbacks.iter().enumerate()
        {
            if sys_handle.entity().to_bits() != reactor_id { continue; }
            let _ = callbacks.remove(idx);

            break;
        }

        // cleanup empty hashmap entries
        if !reactors.is_empty() { return; }
        let _ = self.component_reactors.remove(&comp_id);
    }

    /// Revokes a resource mutation reactor.
    pub(crate) fn revoke_resource_mutation_reactor(&mut self, resource_id: TypeId, reactor_id: u64)
    {
        // get callbacks
        let Some(callbacks) = self.resource_reactors.get_mut(&resource_id) else { return; };

        // revoke reactor
        for (idx, sys_handle) in callbacks.iter().enumerate()
        {
            if sys_handle.entity().to_bits() != reactor_id { continue; }
            let _ = callbacks.remove(idx);
            break;
        }

        // cleanup empty hashmap entries
        if callbacks.len() > 0 { return; }
        let _ = self.resource_reactors.remove(&resource_id);
    }

    /// Revokes an event reactor.
    pub(crate) fn revoke_broadcast_reactor(&mut self, event_id: TypeId, reactor_id: u64)
    {
        // get callbacks
        let Some(callbacks) = self.broadcast_reactors.get_mut(&event_id) else { return; };

        // revoke reactor
        for (idx, sys_handle) in callbacks.iter().enumerate()
        {
            if sys_handle.entity().to_bits() != reactor_id { continue; }
            let _ = callbacks.remove(idx);
            break;
        }

        // cleanup empty hashmap entries
        if callbacks.len() > 0 { return; }
        let _ = self.broadcast_reactors.remove(&event_id);
    }

    /// Revokes an entity event reactor.
    pub(crate) fn revoke_entity_event_reactor(&mut self, entity: Entity, event_id: TypeId, reactor_id: u64)
    {
        // get callbacks
        let Some(callbacks) = self.entity_event_reactors.get_mut(&(entity, event_id)) else { return; };

        // revoke reactor
        for (idx, sys_handle) in callbacks.iter().enumerate()
        {
            if sys_handle.entity().to_bits() != reactor_id { continue; }
            let _ = callbacks.remove(idx);
            break;
        }

        // cleanup empty hashmap entries
        if callbacks.len() > 0 { return; }
        let _ = self.broadcast_reactors.remove(&event_id);
    }

    /// Revokes a despawn reactor.
    pub(crate) fn revoke_despawn_reactor(&mut self, entity: Entity, reactor_id: u64)
    {
        // get callbacks
        let Some(callbacks) = self.despawn_reactors.get_mut(&entity) else { return; };

        // revoke reactor
        for (idx, sys_handle) in callbacks.iter().enumerate()
        {
            if sys_handle.entity().to_bits() != reactor_id { continue; }
            let _ = callbacks.remove(idx);
            break;
        }

        // cleanup empty hashmap entries
        if callbacks.len() > 0 { return; }
        let _ = self.despawn_reactors.remove(&entity);
    }

    /// Queues reactions to a component insertion on an entity.
    pub(crate) fn schedule_insertion_reaction<C: ReactComponent>(
        &mut self,
        commands : &mut Commands,
        entity   : Entity
    ){
        // entity-specific component reactors
        commands.add(
                move |world: &mut World|
                syscall(world, (EntityReactionType::Insertion(TypeId::of::<C>()), entity), schedule_entity_reaction)
            );

        // reaction tree
        // - Must do this before early-outs.
        commands.add(reaction_tree);

        // entity-agnostic component reactors
        let Some(handlers) = self.component_reactors.get(&TypeId::of::<C>()) else { return; };
        for sys_handle in handlers.insertion_callbacks.iter()
        {
            commands.add(
                    ReactionCommand::EntityReaction{
                        reaction_source : entity,
                        reaction_type   : EntityReactionType::Insertion(TypeId::of::<C>()),
                        reactor         : SystemCommand(sys_handle.entity()),
                    }
                );
        }
    }

    /// Queues reactions to a component mutation on an entity.
    pub(crate) fn schedule_mutation_reaction<C: ReactComponent>(
        &mut self,
        commands : &mut Commands,
        entity   : Entity
    ){
        // entity-specific component reactors
        commands.add(
                move |world: &mut World|
                syscall(world, (EntityReactionType::Mutation(TypeId::of::<C>()), entity), schedule_entity_reaction)
            );

        // reaction tree
        // - Must do this before early-outs.
        commands.add(reaction_tree);

        // entity-agnostic component reactors
        let Some(handlers) = self.component_reactors.get(&TypeId::of::<C>()) else { return; };
        for sys_handle in handlers.mutation_callbacks.iter()
        {
            commands.add(
                    ReactionCommand::EntityReaction{
                        reaction_source : entity,
                        reaction_type   : EntityReactionType::Mutation(TypeId::of::<C>()),
                        reactor         : SystemCommand(sys_handle.entity()),
                    }
                );
        }
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
            for entity in buffer.iter()
            {
                // ignore entities that don't exist
                if world.get_entity(*entity).is_none() { continue; }

                // entity-specific component reactors
                if let Ok(entity_reactors) = query.get(world, *entity)
                {
                    schedule_entity_reaction_impl(
                            &mut queue,
                            *entity,
                            EntityReactionType::Removal(checker.component_id),
                            &entity_reactors
                        );
                }

                // entity-agnostic component reactors
                let Some(reactors) = self.component_reactors.get(&checker.component_id) else { continue; };
                for sys_handle in reactors.removal_callbacks.iter()
                {
                    queue.push(
                            ReactionCommand::EntityReaction{
                                reaction_source : *entity,
                                reaction_type   : EntityReactionType::Removal(checker.component_id),
                                reactor         : SystemCommand(sys_handle.entity()),
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

    /// Queues reactions to tracked despawns.
    pub(crate) fn schedule_despawn_reactions(&mut self, world: &mut World)
    {
        while let Some(despawned_entity) = self.despawn_receiver.try_recv()
        {
            let Some(mut despawn_reactors) = self.despawn_reactors.remove(&despawned_entity) else { continue; };

            // queue despawn callbacks
            for sys_handle in despawn_reactors.drain(..)
            {
                let system_entity = sys_handle.entity();
                world.resource_mut::<CobwebCommandQueue<ReactionCommand>>().push(
                        ReactionCommand::Despawn{
                            reaction_source : despawned_entity,
                            reactor         : SystemCommand(system_entity),
                            handle          : sys_handle,
                        }
                    );
            }
        }

        // note: `reaction_tree` is not scheduled here because removals/despawns are handled separately
    }

    /// Queues reactions to a resource mutation.
    pub(crate) fn schedule_resource_mutation_reaction<R: ReactResource>(
        &mut self,
        commands : &mut Commands,
    ){
        let Some(handlers) = self.resource_reactors.get(&TypeId::of::<R>()) else { return; };
        for sys_handle in handlers.iter()
        {
            commands.add(
                ReactionCommand::Resource{
                    reactor: SystemCommand(sys_handle.entity()),
                }
            );
        }
    }

    /// Queues reactions to a broadcasted event.
    pub(crate) fn schedule_broadcast_reaction<E: Send + Sync + 'static>(
        &mut self,
        commands : &mut Commands,
        event    : E,
    ){
        let Some(handlers) = self.broadcast_reactors.get(&TypeId::of::<E>()) else { return; };

        // if there are no handlers, just drop the event data
        let num = handlers.len();
        if num == 0 { return; }

        let data_entity = commands.spawn(BroadcastEventData::new(event)).id();

        for (idx, sys_handle) in handlers.iter().enumerate()
        {
            commands.add(
                ReactionCommand::Event{
                    data_entity,
                    reactor     : SystemCommand(sys_handle.entity()),
                    last_reader : idx + 1 == num,
                }
            );
        }
    }

    /// Queues reactions to an entity event.
    pub(crate) fn schedule_entity_event_reaction<E: Send + Sync + 'static>(
        &mut self,
        commands : &mut Commands,
        target   : Entity,
        event    : E,
    ){
        let Some(handlers) = self.entity_event_reactors.get(&(target, TypeId::of::<E>())) else { return; };

        // if there are no handlers, just drop the event data
        let num = handlers.len();
        if num == 0 { return; }

        let data_entity = commands.spawn(EntityEventData::new(target, event)).id();

        for (idx, sys_handle) in handlers.iter().enumerate()
        {
            commands.add(
                ReactionCommand::Event{
                    data_entity,
                    reactor     : SystemCommand(sys_handle.entity()),
                    last_reader : idx + 1 == num,
                }
            );
        }
    }
}

impl Default for ReactCache
{
    fn default() -> Self
    {
        // prep despawn channel
        let (despawn_sender, despawn_receiver) = new_channel::<Entity>();

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
            resource_reactors     : HashMap::new(),
            broadcast_reactors    : HashMap::new(),
            entity_event_reactors : HashMap::new(),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
