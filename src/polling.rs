// Currently, just helpers for mio library

// TODO: Expand this to become an iternal API for polling operations:
// * sockets (send/recv)
// * timers
// * inter-thread channels
//
// Then we cold implement them either on top of mio-0.6, mio-0.8 or something
// else

use mio_extras::{timer, timer::Timer};

// The default timer has 256 timer wheel slots and capacity
// ("Max number of timeouts that can be in flight at a given time") of 65536.
// This seems to take up a lot of memory.
// Additionally, each timer creates its own background thread, which also takes
// up resources, but changing that would be a major change.
pub fn new_simple_timer<E>() -> Timer<E> {
  timer::Builder::default().num_slots(4).capacity(2).build()
}
