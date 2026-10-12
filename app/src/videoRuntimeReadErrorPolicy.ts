// A read raced an acknowledged project edit; it applies no data. Scheduled
// polling will use the current fence. Other failures remain visible.
export const shouldReportVideoRuntimeReadError = (error: unknown): boolean => {
  const message = String(error).replace(/^Error:\s*/, '');
  return message !== 'Project changed since this video clip runtime read was issued; retry'
    && message !== 'Project changed since this video transition runtime read was issued; retry';
};
