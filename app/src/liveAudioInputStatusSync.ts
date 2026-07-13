export interface LiveAudioInputStatusRequestGate {
  beginCommand: () => number;
  endCommand: (requestEpoch: number) => void;
  beginPoll: () => number | null;
  accepts: (requestEpoch: number) => boolean;
  endPoll: () => void;
  invalidate: () => void;
}

export const createLiveAudioInputStatusRequestGate = (): LiveAudioInputStatusRequestGate => {
  let epoch = 0;
  let pollInFlight = false;
  let commandInFlight = false;

  return {
    beginCommand: () => {
      epoch += 1;
      commandInFlight = true;
      return epoch;
    },
    endCommand: (requestEpoch) => {
      if (requestEpoch === epoch) commandInFlight = false;
    },
    beginPoll: () => {
      if (pollInFlight || commandInFlight) return null;
      pollInFlight = true;
      return epoch;
    },
    accepts: (requestEpoch) => requestEpoch === epoch,
    endPoll: () => {
      pollInFlight = false;
    },
    invalidate: () => {
      epoch += 1;
      commandInFlight = false;
    },
  };
};
