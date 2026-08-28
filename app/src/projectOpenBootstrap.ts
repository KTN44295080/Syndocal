export type ProjectOpenBootstrapAuthorityGate = Readonly<{
  awaitOwnerRegistration: () => Promise<void>;
  refreshAuthority: () => Promise<boolean>;
  authorityReady: () => boolean;
}>;

export const establishProjectOpenBootstrapAuthority = async (
  gate: ProjectOpenBootstrapAuthorityGate,
): Promise<void> => {
  await gate.awaitOwnerRegistration();
  const refreshed = await gate.refreshAuthority();
  if (!refreshed || !gate.authorityReady()) {
    throw new Error(
      "Project authority bootstrap did not produce a current checkpoint; startup project loading was not attempted.",
    );
  }
};

export const isProjectOpenAuthorityMismatch = (error: unknown): boolean =>
  String(error).includes("Project authority changed before mutation");

export type ProjectOpenAuthorityIdentity = Readonly<{
  project_epoch: number;
  project_revision: number;
  checkpoint_hash: string;
}>;

export class ProjectOpenBootstrapRetryLatch {
  private observedKey = "";
  private requested = false;

  observe(authority: ProjectOpenAuthorityIdentity, bootstrapInFlight: boolean): void {
    const key = `${authority.project_epoch}:${authority.project_revision}:${authority.checkpoint_hash}`;
    const changed = key !== this.observedKey;
    this.observedKey = key;
    if (changed && bootstrapInFlight) this.requested = true;
  }

  take(): boolean {
    const requested = this.requested;
    this.requested = false;
    return requested;
  }
}
