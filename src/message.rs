use std::collections::VecDeque;

use crate::component::Component;

pub trait Message: Clone + Send + 'static {}

pub(crate) struct Messages<M: Message> {
    pub(crate) messages: VecDeque<M>,
    pub(crate) capacity: usize,
}
impl<M: Message> Component for Messages<M> {}

/*
- Allow for entity-less segments for messages?
- If the emitter adds the message to its own segment, then receivers can all read from it without requiring 'Clone' from the message type.
- This means that receivers will read from all segments with the message type.
- This also removes the need for a queue and if adding a component to a segment can be made thread-safe, this becomes thread-safe.
- Technically, this would mean that emits and receives could all happen at the same time.
- The emitter that owns the segment will be responsible to reset the count to 0 at the beginning of its execution.
- This works because there is 1 segment per emitter, which means there may be more than 1 segment with the same 'Meta' profile.
*/
