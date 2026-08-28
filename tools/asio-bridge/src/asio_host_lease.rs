//! Process-wide ASIO host/session lease shared by ABI v2 and v3.
//!
//! ASIO exposes one process-global driver.  The lease therefore linearizes
//! enumeration, capability inspection, and lifecycle transitions before any
//! backend (CPAL v2 or the SDK v3 path) is allowed to touch that driver.

use std::sync::atomic::{AtomicU64, Ordering};

const STATE_MASK: u64 = 0xff;
const GENERATION_SHIFT: u32 = 8;

const STOPPED: u8 = 0;
const INSPECTING: u8 = 1;
const STARTING_V2: u8 = 2;
const ACTIVE_V2: u8 = 3;
const STOPPING_V2: u8 = 4;
const FAULT_V2: u8 = 5;
const STARTING_V3: u8 = 6;
const ACTIVE_V3: u8 = 7;
const STOPPING_V3: u8 = 8;
const FAULT_V3: u8 = 9;

static HOST_LEASE: AtomicU64 = AtomicU64::new(pack(0, STOPPED));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Owner {
    V2,
    V3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Ticket {
    generation: u64,
    owner: Owner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BusyState {
    Inspecting,
    Starting(Owner),
    Active(Owner),
    Stopping(Owner),
    Fault(Owner),
}

impl BusyState {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Inspecting => "Inspecting",
            Self::Starting(Owner::V2) => "Starting(v2)",
            Self::Starting(Owner::V3) => "Starting(v3)",
            Self::Active(Owner::V2) => "Active(v2)",
            Self::Active(Owner::V3) => "Active(v3)",
            Self::Stopping(Owner::V2) => "Stopping(v2)",
            Self::Stopping(Owner::V3) => "Stopping(v3)",
            Self::Fault(Owner::V2) => "Fault(v2)",
            Self::Fault(Owner::V3) => "Fault(v3)",
        }
    }
}

const fn pack(generation: u64, state: u8) -> u64 {
    (generation << GENERATION_SHIFT) | state as u64
}

fn generation(value: u64) -> u64 {
    value >> GENERATION_SHIFT
}

fn state(value: u64) -> u8 {
    (value & STATE_MASK) as u8
}

fn busy_state(value: u64) -> Option<BusyState> {
    match state(value) {
        INSPECTING => Some(BusyState::Inspecting),
        STARTING_V2 => Some(BusyState::Starting(Owner::V2)),
        ACTIVE_V2 => Some(BusyState::Active(Owner::V2)),
        STOPPING_V2 => Some(BusyState::Stopping(Owner::V2)),
        FAULT_V2 => Some(BusyState::Fault(Owner::V2)),
        STARTING_V3 => Some(BusyState::Starting(Owner::V3)),
        ACTIVE_V3 => Some(BusyState::Active(Owner::V3)),
        STOPPING_V3 => Some(BusyState::Stopping(Owner::V3)),
        FAULT_V3 => Some(BusyState::Fault(Owner::V3)),
        _ => None,
    }
}

fn owner_state(owner: Owner, v2: u8, v3: u8) -> u8 {
    match owner {
        Owner::V2 => v2,
        Owner::V3 => v3,
    }
}

#[derive(Debug)]
pub(crate) struct Inspection {
    generation: u64,
    completed: bool,
}

impl Inspection {
    pub(crate) fn begin() -> Result<Self, BusyState> {
        loop {
            let current = HOST_LEASE.load(Ordering::Acquire);
            if state(current) != STOPPED {
                return Err(busy_state(current).unwrap_or(BusyState::Inspecting));
            }
            let next_generation = generation(current).wrapping_add(1);
            let next = pack(next_generation, INSPECTING);
            if HOST_LEASE
                .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(Self {
                    generation: next_generation,
                    completed: false,
                });
            }
        }
    }

    pub(crate) fn complete(mut self) {
        let expected = pack(self.generation, INSPECTING);
        let stopped = pack(self.generation, STOPPED);
        let _ = HOST_LEASE.compare_exchange(expected, stopped, Ordering::AcqRel, Ordering::Acquire);
        self.completed = true;
    }
}

impl Drop for Inspection {
    fn drop(&mut self) {
        if !self.completed {
            let expected = pack(self.generation, INSPECTING);
            let stopped = pack(self.generation, STOPPED);
            let _ =
                HOST_LEASE.compare_exchange(expected, stopped, Ordering::AcqRel, Ordering::Acquire);
        }
    }
}

#[derive(Debug)]
pub(crate) struct StartGuard {
    ticket: Ticket,
    resolved: bool,
}

impl StartGuard {
    pub(crate) fn begin(owner: Owner) -> Result<Self, BusyState> {
        loop {
            let current = HOST_LEASE.load(Ordering::Acquire);
            if state(current) != STOPPED {
                return Err(busy_state(current).unwrap_or(BusyState::Inspecting));
            }
            let next_generation = generation(current).wrapping_add(1);
            let starting = owner_state(owner, STARTING_V2, STARTING_V3);
            if HOST_LEASE
                .compare_exchange(
                    current,
                    pack(next_generation, starting),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return Ok(Self {
                    ticket: Ticket {
                        generation: next_generation,
                        owner,
                    },
                    resolved: false,
                });
            }
        }
    }

    pub(crate) fn activate(mut self) -> Ticket {
        let starting = owner_state(self.ticket.owner, STARTING_V2, STARTING_V3);
        let active = owner_state(self.ticket.owner, ACTIVE_V2, ACTIVE_V3);
        HOST_LEASE
            .compare_exchange(
                pack(self.ticket.generation, starting),
                pack(self.ticket.generation, active),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .expect("ASIO start lease changed before activation");
        self.resolved = true;
        self.ticket
    }
}

impl Drop for StartGuard {
    fn drop(&mut self) {
        if !self.resolved {
            let starting = owner_state(self.ticket.owner, STARTING_V2, STARTING_V3);
            let _ = HOST_LEASE.compare_exchange(
                pack(self.ticket.generation, starting),
                pack(self.ticket.generation, STOPPED),
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }
}

pub(crate) fn begin_stop(ticket: Ticket) -> Result<(), BusyState> {
    let active = owner_state(ticket.owner, ACTIVE_V2, ACTIVE_V3);
    let fault = owner_state(ticket.owner, FAULT_V2, FAULT_V3);
    let stopping = owner_state(ticket.owner, STOPPING_V2, STOPPING_V3);
    for expected_state in [active, fault] {
        if HOST_LEASE
            .compare_exchange(
                pack(ticket.generation, expected_state),
                pack(ticket.generation, stopping),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            return Ok(());
        }
    }
    let current = HOST_LEASE.load(Ordering::Acquire);
    if current == pack(ticket.generation, stopping) {
        return Ok(());
    }
    Err(busy_state(current).unwrap_or(BusyState::Inspecting))
}

pub(crate) fn stop_succeeded(ticket: Ticket) {
    let stopping = owner_state(ticket.owner, STOPPING_V2, STOPPING_V3);
    let _ = HOST_LEASE.compare_exchange(
        pack(ticket.generation, stopping),
        pack(ticket.generation, STOPPED),
        Ordering::AcqRel,
        Ordering::Acquire,
    );
}

pub(crate) fn stop_failed(ticket: Ticket) {
    let stopping = owner_state(ticket.owner, STOPPING_V2, STOPPING_V3);
    let fault = owner_state(ticket.owner, FAULT_V2, FAULT_V3);
    let _ = HOST_LEASE.compare_exchange(
        pack(ticket.generation, stopping),
        pack(ticket.generation, fault),
        Ordering::AcqRel,
        Ordering::Acquire,
    );
}

pub(crate) fn mark_fault(ticket: Ticket) {
    let active = owner_state(ticket.owner, ACTIVE_V2, ACTIVE_V3);
    let starting = owner_state(ticket.owner, STARTING_V2, STARTING_V3);
    let fault = owner_state(ticket.owner, FAULT_V2, FAULT_V3);
    for expected_state in [active, starting] {
        if HOST_LEASE
            .compare_exchange(
                pack(ticket.generation, expected_state),
                pack(ticket.generation, fault),
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            return;
        }
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
pub(crate) fn is_fault(ticket: Ticket) -> bool {
    let fault = owner_state(ticket.owner, FAULT_V2, FAULT_V3);
    HOST_LEASE.load(Ordering::Acquire) == pack(ticket.generation, fault)
}

#[cfg(test)]
pub(crate) fn current_state() -> Option<BusyState> {
    busy_state(HOST_LEASE.load(Ordering::Acquire))
}

#[cfg(test)]
pub(crate) fn reset_for_test() {
    HOST_LEASE.store(pack(0, STOPPED), Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc, Barrier, Mutex};

    static TEST_LEASE: Mutex<()> = Mutex::new(());

    #[test]
    fn starts_are_mutually_exclusive_and_stale_tickets_cannot_release_owner() {
        let _test_lease = TEST_LEASE.lock().unwrap();
        reset_for_test();
        let v2 = StartGuard::begin(Owner::V2).unwrap().activate();
        assert_eq!(
            StartGuard::begin(Owner::V3).unwrap_err(),
            BusyState::Active(Owner::V2)
        );
        begin_stop(v2).unwrap();
        stop_succeeded(v2);

        let v3 = StartGuard::begin(Owner::V3).unwrap().activate();
        stop_succeeded(v2);
        assert_eq!(current_state(), Some(BusyState::Active(Owner::V3)));
        begin_stop(v3).unwrap();
        stop_succeeded(v3);
        assert_eq!(current_state(), None);
    }

    #[test]
    fn inspection_blocks_both_versions_and_fault_requires_explicit_drain() {
        let _test_lease = TEST_LEASE.lock().unwrap();
        reset_for_test();
        let inspection = Inspection::begin().unwrap();
        assert_eq!(
            StartGuard::begin(Owner::V2).unwrap_err(),
            BusyState::Inspecting
        );
        inspection.complete();

        let ticket = StartGuard::begin(Owner::V3).unwrap().activate();
        mark_fault(ticket);
        assert_eq!(current_state(), Some(BusyState::Fault(Owner::V3)));
        assert_eq!(
            Inspection::begin().unwrap_err(),
            BusyState::Fault(Owner::V3)
        );
        begin_stop(ticket).unwrap();
        stop_succeeded(ticket);
        assert_eq!(current_state(), None);
    }

    #[test]
    fn failed_stop_remains_fault_and_does_not_admit_fallback() {
        let _test_lease = TEST_LEASE.lock().unwrap();
        reset_for_test();
        let ticket = StartGuard::begin(Owner::V2).unwrap().activate();
        begin_stop(ticket).unwrap();
        stop_failed(ticket);
        assert_eq!(current_state(), Some(BusyState::Fault(Owner::V2)));
        assert!(StartGuard::begin(Owner::V3).is_err());
        begin_stop(ticket).unwrap();
        stop_succeeded(ticket);
        assert_eq!(current_state(), None);
    }

    #[test]
    fn concurrent_cross_version_starts_admit_exactly_one_owner() {
        let _test_lease = TEST_LEASE.lock().unwrap();
        reset_for_test();
        const CONTENDERS: usize = 16;
        let begin = Arc::new(Barrier::new(CONTENDERS + 1));
        let release = Arc::new(Barrier::new(CONTENDERS + 1));
        let (send, receive) = mpsc::channel();
        let mut threads = Vec::with_capacity(CONTENDERS);
        for index in 0..CONTENDERS {
            let begin = Arc::clone(&begin);
            let release = Arc::clone(&release);
            let send = send.clone();
            threads.push(std::thread::spawn(move || {
                begin.wait();
                let ticket = StartGuard::begin(if index % 2 == 0 { Owner::V2 } else { Owner::V3 })
                    .ok()
                    .map(StartGuard::activate);
                send.send(ticket).unwrap();
                release.wait();
                if let Some(ticket) = ticket {
                    begin_stop(ticket).unwrap();
                    stop_succeeded(ticket);
                }
            }));
        }
        drop(send);
        begin.wait();
        let results: Vec<_> = receive.iter().take(CONTENDERS).collect();
        assert_eq!(results.iter().filter(|ticket| ticket.is_some()).count(), 1);
        release.wait();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(current_state(), None);
    }

    #[test]
    fn inspection_rejects_every_v2_v3_owning_state() {
        let _test_lease = TEST_LEASE.lock().unwrap();
        for owner in [Owner::V2, Owner::V3] {
            reset_for_test();
            let guard = StartGuard::begin(owner).unwrap();
            assert_eq!(Inspection::begin().unwrap_err(), BusyState::Starting(owner));
            let ticket = guard.activate();
            assert_eq!(Inspection::begin().unwrap_err(), BusyState::Active(owner));
            mark_fault(ticket);
            assert_eq!(Inspection::begin().unwrap_err(), BusyState::Fault(owner));
            begin_stop(ticket).unwrap();
            assert_eq!(Inspection::begin().unwrap_err(), BusyState::Stopping(owner));
            stop_succeeded(ticket);
            Inspection::begin().unwrap().complete();
        }
        assert_eq!(current_state(), None);
    }
}
