// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Instrument typestate reachability checker.
//!
//! For each instrument declaration, verifies:
//! 1. Every declared state is reachable from the initial state.
//! 2. Every non-terminal state has at least one path to a terminal
//!    state (`Settled` or `Void` or a custom terminal).

use std::collections::{HashMap, HashSet, VecDeque};

use oikos_syntax::{instrument::InstrumentState, Model};

use crate::error::CheckError;

/// Check all instrument typestate machines for reachability.
pub fn check(model: &Model) -> Result<(), Vec<CheckError>> {
    let mut errors = Vec::new();

    for inst in &model.instruments {
        let reachable = reachable_states(inst);

        // Check 1: all declared states are reachable.
        for state in &inst.states {
            if !reachable.contains(state) {
                errors.push(CheckError::UnreachableState {
                    instrument: inst.name.to_string(),
                    state: format!("{state:?}"),
                    span: inst.span,
                });
            }
        }

        // Check 2: every non-terminal state can reach a terminal.
        let terminals: HashSet<&InstrumentState> =
            inst.states.iter().filter(|s| is_terminal(s)).collect();

        for state in inst.states.iter().filter(|s| !is_terminal(s)) {
            if !can_reach_terminal(state, &terminals, inst) {
                errors.push(CheckError::NonTerminatingState {
                    instrument: inst.name.to_string(),
                    state: format!("{state:?}"),
                    span: inst.span,
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Compute the set of states reachable from the instrument's initial state
/// by BFS over declared transitions.
fn reachable_states(inst: &oikos_syntax::instrument::InstrumentDecl) -> HashSet<InstrumentState> {
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(inst.initial.clone());

    while let Some(state) = queue.pop_front() {
        if reachable.insert(state.clone()) {
            for t in inst.transitions.iter().filter(|t| t.from == state) {
                queue.push_back(t.to.clone());
            }
        }
    }

    reachable
}

/// A state is terminal if no further transitions depart from it.
/// `Settled` and `Void` are always terminal by convention.
fn is_terminal(state: &InstrumentState) -> bool {
    matches!(state, InstrumentState::Settled | InstrumentState::Void)
}

/// Check whether `from` can reach any terminal state by forward BFS.
fn can_reach_terminal(
    from: &InstrumentState,
    terminals: &HashSet<&InstrumentState>,
    inst: &oikos_syntax::instrument::InstrumentDecl,
) -> bool {
    // Build forward adjacency.
    let mut adj: HashMap<&InstrumentState, Vec<&InstrumentState>> = HashMap::new();
    for t in &inst.transitions {
        adj.entry(&t.from).or_default().push(&t.to);
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(from);

    while let Some(state) = queue.pop_front() {
        if terminals.contains(state) {
            return true;
        }
        if visited.insert(state) {
            if let Some(neighbours) = adj.get(state) {
                queue.extend(neighbours.iter());
            }
        }
    }

    false
}
