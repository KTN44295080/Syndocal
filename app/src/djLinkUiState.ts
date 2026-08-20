import type { DjLinkRuntimeStatus, RemoteControlStatus } from "./types";

export interface DjLinkSecretViewState {
  token: string | null;
  copied: boolean;
}

export const emptyDjLinkRuntimeStatus = (): DjLinkRuntimeStatus => ({
  available: false,
  connected: false,
  peer: null,
  generation: 0,
  master: false,
  trackActive: false,
  loopDivision: null,
  released: false,
  lastEventId: null,
  ageMs: null,
});

export const unavailableRemoteControlStatus = (): RemoteControlStatus => ({
  running: false,
  active_connections: 0,
  rejected_connections: 0,
  clients: [],
  dj_link: emptyDjLinkRuntimeStatus(),
});

export const availableRemoteControlStatus = (
  status: RemoteControlStatus,
): RemoteControlStatus => ({
  ...status,
  dj_link: status.dj_link
    ? { ...status.dj_link, available: true }
    : emptyDjLinkRuntimeStatus(),
});

/**
 * Project one status poll into authority state.  A rejected IPC poll is an
 * authority loss, not a transient display update: clear the running bit and
 * every retained DJ Link field so the operator cannot use stale metadata.
 */
export const projectRemoteControlStatusPoll = (
  status: RemoteControlStatus | null,
): { running: boolean; status: RemoteControlStatus } => {
  if (!status) {
    return { running: false, status: unavailableRemoteControlStatus() };
  }
  return { running: status.running, status: availableRemoteControlStatus(status) };
};

export const clearedDjLinkSecret = (copied = false): DjLinkSecretViewState => ({
  token: null,
  copied,
});
