//! sluice's host process (135, 129, 71, 150; rules core.31, core.25,
//! core.49; record 0093): the one process for each user on a host that
//! moves every repository's queues, each apart, holds the host's bound on
//! runs, and reads each repository it holds for the watches and waits
//! that follow it. A command that needs the host's queues moved reaches it
//! or starts it; it ends once nothing is in play and nothing follows it,
//! and keeps nothing the records and the host's state do not hold.

pub mod room;
