//! Admission ordering for role/lease publication and denying lifecycle boundaries.
//!
//! Reservations own logical admission, never a mutex guard. A denying boundary
//! invalidates queued tickets before draining already admitted publications. It
//! then retains exclusion through managed retirement and Standby/project commit.
//! All waits happen before application/domain locks; no mutex crosses I/O or joins.

use protocol::control_plane_command::OutputControlErrorCodeV2;
use std::sync::{Condvar, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrderingError {
    Superseded,
    Unavailable,
}

impl OrderingError {
    pub(crate) fn code(self) -> OutputControlErrorCodeV2 {
        match self {
            Self::Superseded => OutputControlErrorCodeV2::StaleFence,
            Self::Unavailable => OutputControlErrorCodeV2::Internal,
        }
    }
}

impl std::fmt::Display for OrderingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Superseded => "Output role reservation was superseded by Standby, replacement, or main-window close; explicitly Arm again with fresh authority",
            Self::Unavailable => "Output role admission authority is unavailable; output remains denied",
        })
    }
}

#[derive(Default)]
struct State {
    generation: u64,
    active: usize,
    pending: usize,
    boundary_owned: bool,
    closed: bool,
}

#[derive(Default)]
pub(crate) struct OutputRoleOrdering {
    state: Mutex<State>,
    changed: Condvar,
}

#[derive(Clone, Copy)]
pub(crate) struct RoleTicket(u64);

pub(crate) struct RolePublication<'a> {
    ordering: &'a OutputRoleOrdering,
    generation: u64,
}

pub(crate) struct DenyReservation<'a> {
    ordering: &'a OutputRoleOrdering,
}

impl OutputRoleOrdering {
    pub(crate) fn capture(&self) -> Result<RoleTicket, OrderingError> {
        let state = self.state.lock().map_err(|_| OrderingError::Unavailable)?;
        if state.closed || state.pending != 0 {
            return Err(OrderingError::Superseded);
        }
        Ok(RoleTicket(state.generation))
    }

    pub(crate) fn enter(&self, ticket: RoleTicket) -> Result<RolePublication<'_>, OrderingError> {
        let mut state = self.state.lock().map_err(|_| OrderingError::Unavailable)?;
        if state.closed || state.pending != 0 || state.generation != ticket.0 {
            return Err(OrderingError::Superseded);
        }
        state.active = state
            .active
            .checked_add(1)
            .ok_or(OrderingError::Unavailable)?;
        Ok(RolePublication {
            ordering: self,
            generation: ticket.0,
        })
    }

    /// Call with no domain locks. Pending denial is visible before the wait.
    pub(crate) fn deny(&self) -> Result<DenyReservation<'_>, OrderingError> {
        let mut state = self.state.lock().map_err(|_| OrderingError::Unavailable)?;
        state.generation = match state.generation.checked_add(1) {
            Some(next) => next,
            None => {
                state.closed = true;
                return Err(OrderingError::Unavailable);
            }
        };
        state.pending = state
            .pending
            .checked_add(1)
            .ok_or(OrderingError::Unavailable)?;
        self.changed.notify_all();
        while state.active != 0 || state.boundary_owned {
            state = self
                .changed
                .wait(state)
                .map_err(|_| OrderingError::Unavailable)?;
        }
        state.boundary_owned = true;
        Ok(DenyReservation { ordering: self })
    }

    /// Main-window destruction seals queued and future Arm before inspecting
    /// current output state. An in-flight publication requires shutdown too.
    pub(crate) fn seal(&self) -> Result<bool, OrderingError> {
        let mut state = self.state.lock().map_err(|_| OrderingError::Unavailable)?;
        state.closed = true;
        self.changed.notify_all();
        Ok(state.active != 0)
    }

    #[cfg(test)]
    pub(crate) fn wait_until_denial_pending(&self) {
        let mut state = self.state.lock().unwrap();
        while state.pending == 0 {
            state = self.changed.wait(state).unwrap();
        }
    }
}

impl RolePublication<'_> {
    pub(crate) fn validate(&self) -> Result<(), OrderingError> {
        let state = self
            .ordering
            .state
            .lock()
            .map_err(|_| OrderingError::Unavailable)?;
        if state.closed || state.generation != self.generation || state.pending != 0 {
            Err(OrderingError::Superseded)
        } else {
            Ok(())
        }
    }
}

impl Drop for RolePublication<'_> {
    fn drop(&mut self) {
        let mut state = self
            .ordering
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        state.active -= 1;
        self.ordering.changed.notify_all();
    }
}

impl Drop for DenyReservation<'_> {
    fn drop(&mut self) {
        let mut state = self
            .ordering
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        state.boundary_owned = false;
        state.pending -= 1;
        self.ordering.changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn boundary_invalidates_queued_and_overlapping_tickets_until_explicit_rearm() {
        let ordering = OutputRoleOrdering::default();
        let old = ordering.capture().unwrap();
        let deny = ordering.deny().unwrap();
        assert!(matches!(
            ordering.enter(old),
            Err(OrderingError::Superseded)
        ));
        assert!(matches!(ordering.capture(), Err(OrderingError::Superseded)));
        drop(deny);
        assert!(matches!(
            ordering.enter(old),
            Err(OrderingError::Superseded)
        ));
        assert!(ordering.enter(ordering.capture().unwrap()).is_ok());
        assert_eq!(
            OrderingError::Superseded.code(),
            OutputControlErrorCodeV2::StaleFence
        );
    }

    #[test]
    fn pending_boundary_drains_durable_installation_without_holding_a_mutex() {
        let ordering = OutputRoleOrdering::default();
        let publication = ordering.enter(ordering.capture().unwrap()).unwrap();
        let (done_tx, done_rx) = mpsc::channel();
        std::thread::scope(|scope| {
            let boundary = scope.spawn(|| {
                let _deny = ordering.deny().unwrap();
                done_tx.send(()).unwrap();
            });
            // Observe the pending transition through its condition variable,
            // never a timer or polling loop.
            let mut state = ordering.state.lock().unwrap();
            while state.pending == 0 {
                state = ordering.changed.wait(state).unwrap();
            }
            drop(state);
            assert_eq!(publication.validate(), Err(OrderingError::Superseded));
            assert!(done_rx.try_recv().is_err());
            drop(publication);
            boundary.join().unwrap();
            done_rx.recv().unwrap();
        });
    }

    #[test]
    fn main_close_seals_idle_and_inflight_enable_permanently() {
        let ordering = OutputRoleOrdering::default();
        let queued = ordering.capture().unwrap();
        assert!(!ordering.seal().unwrap());
        assert!(matches!(
            ordering.enter(queued),
            Err(OrderingError::Superseded)
        ));
        drop(ordering.deny().unwrap());
        assert!(matches!(ordering.capture(), Err(OrderingError::Superseded)));
        let ordering = OutputRoleOrdering::default();
        let active = ordering.enter(ordering.capture().unwrap()).unwrap();
        assert!(ordering.seal().unwrap());
        assert_eq!(active.validate(), Err(OrderingError::Superseded));
    }
}
