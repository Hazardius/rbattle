//! Scheduling game play.

use state::Player;
use state::{Action, State};

use futures::sync::oneshot;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem::replace;

/// A `Scheduler` collects actions from all players, and then broadcasts the
/// full list once everyone has submitted their moves for that turn.
///
/// When a player submits their moves, they provide a
/// `oneshot` future on which `Scheduler` should send the full move list once it
/// is available.
struct Scheduler {
    // The number of the last turn we broadcast out.
    turn: usize,

    // The number of players that have actually joined.
    joined: usize,

    // A scheduler actually maintains its own copy of the game state, for
    // generating checksums to send to clients.
    state: State,

    // A vector recording submitted actions and reply channels; the `i`'th
    // element is for `Player(i)`. Once this has actions for every joined player,
    // we apply all the actions to our state in a given order, compute the new
    // state's checksum, and then transmit the collected moves to all the
    // players.
    pending_actions: Vec<Option<(PlayerActions, oneshot::Sender<CollectedMoves>)>>
}

impl Scheduler {
    fn new(initial_state: State) -> Scheduler {
        let num_players = initial_state.map.player_colors.len();
        let mut pending_actions = Vec::new();
        for _ in 0..num_players {
            pending_actions.push(None)
        }
        Scheduler { turn: 0, joined: 0, state: initial_state, pending_actions }
    }

    // Add another player to the game. Return its number, or `None` if there is
    // no room for more players.
    fn player_join(&mut self) -> Option<usize> {
        if self.joined >= self.pending_actions.len() {
            None
        } else {
            self.joined += 1;
            Some(self.joined - 1)
        }
    }

    fn submit_actions(&mut self,
                      actions: PlayerActions,
                      reply_to: oneshot::Sender<CollectedMoves>) {
        assert_eq!(actions.turn, self.turn);
        assert!(self.pending_actions[actions.player.0].is_none());
        let player = actions.player.0;
        self.pending_actions[player] = Some((actions, reply_to));

        // Have all the players that have joined finally submitted an action?
        if self.pending_actions.iter().take(self.joined).all(|o| o.is_some()) {
            // Grab the list of pending actions and reset it for the next turn.
            let pendings = replace(&mut self.pending_actions, vec![]);

            // Collect all the actions into a single vector,
            // collect all the reply-to's in another vector,
            // and apply all the actions to our state.
            let mut collected_reply_tos = Vec::new();
            let mut collected_actions = Vec::new();

            for player in pendings {
                let (player_actions, reply_to) = player.unwrap();
                for action in player_actions.actions {
                    self.state.take_action(&action);
                    collected_actions.push(action);
                }
                collected_reply_tos.push(reply_to);
            }

            // Compute a checksum for the resulting state.
            let mut hasher = DefaultHasher::new();
            self.state.hash(&mut hasher);
            let state_checksum = hasher.finish();

            // We are now in the new turn.
            self.turn += 1;

            let collected = CollectedMoves {
                turn: self.turn,
                actions: collected_actions,
                state_checksum
            };

            // Broadcast out the new state of the world to all players.
            for reply_to in collected_reply_tos {
                reply_to.send(collected.clone())
                    .expect("sending collected moves failed");
            }
        }
    }
}




/// A set of actions submitted by a single player on a single turn.
#[derive(Clone, Debug)]
struct PlayerActions {
    // The player submitting these actions.
    player: Player,

    // The turn number they believe they're on.
    turn: usize,

    // The actions they wish to submit.
    actions: Vec<Action>,
}

/// A collection of all moves submitted by all players.
#[derive(Clone, Debug)]
struct CollectedMoves {
    // The turn these moves produce when applied.
    turn: usize,

    // The actions to apply to the prior state.
    actions: Vec<Action>,

    // The hash value of the State that should result, as a checksum.
    state_checksum: u64
}