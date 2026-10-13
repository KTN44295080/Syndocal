// Operator-facing names for the Show DMX preparation stages. The stage IDs
// stay stable for diagnostics, error details and the setup contract.
const stageLabels: Record<string, string> = {
  "1/4 output role Both": "Taking output control (1/4)",
  "2/4 optional machine-local binding": "Confirming USB-DMX device (2/4)",
  "3/4 Art-Net loopback": "Starting DMX output (3/4)",
  "4/4 optional Open DMX arm": "Starting USB-DMX (4/4)",
};

export const showDmxStageLabel = (stage: string) => stageLabels[stage] ?? stage;
