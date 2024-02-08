//local shortcuts

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::collections::VecDeque;

//-------------------------------------------------------------------------------------------------------------------

/// Buffers queued cobweb commands of type `T`.
#[derive(Resource)]
pub(crate) struct CobwebCommandQueue<T: Send + Sync + 'static>
{
    /// Queued commands.
    commands: VecDeque<T>,

    /// Cached buffers for storing commands.
    buffers: Vec<VecDeque<T>>,
}

impl<T: Send + Sync + 'static> CobwebCommandQueue<T>
{
    /// Removes the inner command queue.
    pub(crate) fn remove(&mut self) -> VecDeque<T>
    {
        let replacement = self.buffers.pop().unwrap_or_default();
        std::mem::replace(&mut self.commands, replacement)
    }

    /// Adds a cobweb command to the end of the queue.
    pub(crate) fn push(&mut self, command: T)
    {
        self.commands.push_back(command);
    }

    /// Removes a command from the front of the queue.
    pub(crate) fn pop_front(&mut self) -> Option<T>
    {
        self.commands.pop_front()
    }

    /// Pushes a list of cobweb commands to the end of the command queue.
    pub(crate) fn append(&mut self, mut new: VecDeque<T>)
    {
        if new.len() > 0
        {
            self.commands.append(&mut new);
        }
        self.buffers.push(new);
    }

    /// Pushes a list of cobweb commands to the end of the command queue then returns the inner queue.
    pub(crate) fn append_and_remove(&mut self, mut new: VecDeque<T>) -> VecDeque<T>
    {
        if new.len() > 0
        {
            self.commands.append(&mut new);
        }
        self.buffers.push(new);
        self.remove()
    }
}

impl<T: Send + Sync + 'static> Default for CobwebCommandQueue<T>
{
    fn default() -> Self
    {
        Self{
            commands: VecDeque::default(),
            buffers: Vec::default()
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
