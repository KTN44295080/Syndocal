async function remoteDisclosureScrollInPage() {
  const remoteZone = document.querySelector('[data-io-zone="remote"]');
  const remoteControl = remoteZone?.querySelector('.remoteControl');
  const disclosureStack = remoteControl?.querySelector(':scope > .ioDisclosureStack');
  const webRemoteDisclosure = disclosureStack?.querySelector(':scope > [data-io-disclosure="web-remote"]') ?? null;
  const webRemoteBody = webRemoteDisclosure?.querySelector(':scope > [data-io-disclosure-body]') ?? null;
  const serverDesk = webRemoteBody?.querySelector(':scope > .remoteServerDesk');
  const serverHeader = serverDesk?.querySelector(':scope > .ioDeskHeader');
  const remoteAction = serverDesk?.querySelector('[data-io-control="remote-start"], [data-io-control="remote-stop"]');
  const disclosureNames = ['web-remote', 'remote-security', 'remote-endpoints', 'dj-link', 'remote-standby'];
  const disclosures = disclosureNames.map((name) =>
    remoteControl?.querySelector(`[data-io-disclosure="${name}"]`) ?? null,
  );
  const djLinkControl = remoteControl?.querySelector('[data-io-control="dj-link-wired-binding"]');
  const djLinkRefreshControl = remoteControl?.querySelector('[data-io-control="dj-link-refresh-wired-candidates"]') ?? null;
  const djLinkArmControl = remoteControl?.querySelector('[data-io-control="dj-link-arm"]') ?? null;
  const djLinkDisarmControl = remoteControl?.querySelector('[data-io-control="dj-link-disarm"]') ?? null;
  const djLinkRotateControl = remoteControl?.querySelector('[data-io-control="dj-link-rotate-token"]') ?? null;
  // Arm and Disarm are intentionally mutually exclusive. The armed browser
  // fixture proves Refresh/Disarm/Rotate; source-level control inventory
  // covers the unarmed Arm branch. Treating both as required in one DOM state
  // would incorrectly fail the conditional control contract.
  const djLinkArmOrDisarmControl = djLinkArmControl ?? djLinkDisarmControl;
  const djLinkActionControls = [
    djLinkRefreshControl,
    djLinkArmOrDisarmControl,
    djLinkRotateControl,
  ];
  const sizeOf = (element) => {
    if (!element) return null;
    const rect = element.getBoundingClientRect();
    return { width: rect.width, height: rect.height };
  };
  const visibleWithin = (element, container) => {
    if (!element || !container) return false;
    const elementRect = element.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    const style = getComputedStyle(element);
    return (
      elementRect.width > 0 &&
      elementRect.height > 0 &&
      style.display !== 'none' &&
      style.visibility !== 'hidden' &&
      elementRect.left >= containerRect.left - 1 &&
      elementRect.right <= containerRect.right + 1 &&
      elementRect.top >= containerRect.top - 1 &&
      elementRect.bottom <= containerRect.bottom + 1
      );
  };
  const hitTestableAtCenter = (element) => {
    if (!element) return false;
    const rect = element.getBoundingClientRect();
    const target = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
    return target === element || element.contains(target);
  };
  const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const revealInScrollport = (element, scrollport) => {
    if (!(element instanceof HTMLElement) || !(scrollport instanceof HTMLElement)) return;
    const target = element.getBoundingClientRect();
    const container = scrollport.getBoundingClientRect();
    if (target.top < container.top) {
      scrollport.scrollTop += target.top - container.top - 2;
    } else if (target.bottom > container.bottom) {
      scrollport.scrollTop += target.bottom - container.bottom + 2;
    }
  };
  const revealInDisclosureStack = async (element) => {
    if (!(element instanceof HTMLElement) || !(disclosureStack instanceof HTMLElement)) return;
    // Endpoint controls live in a second, bounded scrollport. Scroll the
    // innermost owner first, then the disclosure stack; `scrollIntoView`
    // alone can instead move the fixed workspace at compact heights.
    revealInScrollport(element, element.closest('.remoteEndpointDesk'));
    await settle();
    revealInScrollport(element, disclosureStack);
    await settle();
  };

  // Web Remote now owns the server desk inside the shared stack. Open that
  // peer disclosure explicitly before measuring its header/action reachability.
  const openDisclosure = (disclosure) => {
    const summary = disclosure?.querySelector(':scope > summary');
    if (summary instanceof HTMLElement && !disclosure.open) summary.click();
  };
  if (remoteControl instanceof HTMLElement) remoteControl.scrollTop = 0;
  if (serverDesk instanceof HTMLElement) serverDesk.scrollTop = 0;
  if (disclosureStack instanceof HTMLElement) disclosureStack.scrollTop = 0;
  for (const endpointDesk of remoteControl?.querySelectorAll('.remoteEndpointDesk') ?? []) {
    if (endpointDesk instanceof HTMLElement) endpointDesk.scrollTop = 0;
  }
  openDisclosure(webRemoteDisclosure);
  for (const disclosure of disclosures) {
    openDisclosure(disclosure);
  }
  await settle();

  const remoteActionSizeBeforeDisclosureScroll = sizeOf(remoteAction);
  const allDisclosuresOpen = disclosures.every((disclosure) => disclosure?.open === true);
  const remoteStyle = remoteControl ? getComputedStyle(remoteControl) : null;
  const disclosureStackStyle = disclosureStack ? getComputedStyle(disclosureStack) : null;
  const disclosureStackScrollable = Boolean(
    disclosureStack instanceof HTMLElement &&
    disclosureStack.scrollHeight > disclosureStack.clientHeight,
  );
  if (disclosureStack instanceof HTMLElement) {
    disclosureStack.scrollTop = disclosureStack.scrollHeight;
  }
  await settle();
  // Web Remote now scrolls in the same stack as DJ Link. Reveal its critical
  // controls before moving the stack back to the lower DJ Link surface.
  await revealInDisclosureStack(serverHeader);
  await revealInDisclosureStack(remoteAction);
  const serverHeaderReachableAfterDisclosureScroll = visibleWithin(serverHeader, remoteZone);
  const remoteActionReachableAfterDisclosureScroll = visibleWithin(remoteAction, remoteZone);
  const remoteActionHitTestableAfterDisclosureScroll = hitTestableAtCenter(remoteAction);

  if (disclosureStack instanceof HTMLElement) {
    disclosureStack.scrollTop = disclosureStack.scrollHeight;
  }
  await settle();
  await revealInDisclosureStack(djLinkControl);
  const djLinkActionControlReachability = [];
  for (const control of djLinkActionControls) {
    await revealInDisclosureStack(control);
    const endpointDesk = control?.closest('.remoteEndpointDesk') ?? null;
    const controlRect = control?.getBoundingClientRect();
    const endpointRect = endpointDesk?.getBoundingClientRect();
    const hitTarget = controlRect
      ? document.elementFromPoint(controlRect.left + controlRect.width / 2, controlRect.top + controlRect.height / 2)
      : null;
    djLinkActionControlReachability.push({
      control: control?.getAttribute('data-io-control') ?? null,
      reachable:
        visibleWithin(control, remoteZone) &&
        visibleWithin(control, disclosureStack) &&
        visibleWithin(control, endpointDesk),
      hitTestable: hitTestableAtCenter(control),
      controlBounds: controlRect ? [controlRect.top, controlRect.bottom] : null,
      endpointBounds: endpointRect ? [endpointRect.top, endpointRect.bottom] : null,
      endpointScrollTop: endpointDesk instanceof HTMLElement ? endpointDesk.scrollTop : null,
      disclosureBounds: disclosureStack instanceof HTMLElement
        ? [disclosureStack.getBoundingClientRect().top, disclosureStack.getBoundingClientRect().bottom]
        : null,
      hitTargetControl: hitTarget?.getAttribute('data-io-control') ?? null,
      hitTargetClass: typeof hitTarget?.className === 'string' ? hitTarget.className : null,
    });
  }
  // Action reveals may move both nested scrollports. Re-reveal the binding
  // immediately before measuring it so its own reachability/hit proof is not
  // inherited from the last action control's scroll position.
  await revealInDisclosureStack(djLinkControl);
  const djLinkEndpointDesk = djLinkControl?.closest('.remoteEndpointDesk') ?? null;
  const remoteActionSizeAfterDisclosureScroll = sizeOf(remoteAction);
  const remoteActionSizePreserved = Boolean(
    remoteActionSizeBeforeDisclosureScroll &&
    remoteActionSizeAfterDisclosureScroll &&
    Math.abs(remoteActionSizeBeforeDisclosureScroll.width - remoteActionSizeAfterDisclosureScroll.width) <= 0.5 &&
    Math.abs(remoteActionSizeBeforeDisclosureScroll.height - remoteActionSizeAfterDisclosureScroll.height) <= 0.5,
  );

  const result = {
    remoteFound: remoteControl instanceof HTMLElement,
    serverDeskFound: serverDesk instanceof HTMLElement,
    serverHeaderFound: serverHeader instanceof HTMLElement,
    remoteActionFound: remoteAction instanceof HTMLElement,
    disclosureStackFound: disclosureStack instanceof HTMLElement,
    djLinkControlFound: djLinkControl instanceof HTMLElement,
    djLinkArmControlFound: djLinkArmControl instanceof HTMLElement,
    djLinkDisarmControlFound: djLinkDisarmControl instanceof HTMLElement,
    djLinkArmOrDisarmExclusive: (djLinkArmControl instanceof HTMLElement) !== (djLinkDisarmControl instanceof HTMLElement),
    djLinkActionControlsFound: djLinkActionControls.every((control) => control instanceof HTMLElement),
    allDisclosuresOpen,
    remoteOwnsNoVerticalScroll: remoteStyle?.overflowY === 'hidden',
    disclosureStackOwnsVerticalScroll: disclosureStackStyle?.overflowY === 'auto' && disclosureStackScrollable,
    remoteScrollTopAfterDisclosureScroll: remoteControl instanceof HTMLElement ? remoteControl.scrollTop : -1,
    disclosureStackScrollTopAfterDisclosureScroll:
      disclosureStack instanceof HTMLElement ? disclosureStack.scrollTop : -1,
    remoteActionSizeBeforeDisclosureScroll,
    remoteActionSizeAfterDisclosureScroll,
    remoteActionSizePreserved,
    serverHeaderReachableAfterDisclosureScroll,
    remoteActionReachableAfterDisclosureScroll,
    remoteActionHitTestableAfterDisclosureScroll,
    djLinkControlReachableAfterDisclosureScroll:
      visibleWithin(djLinkControl, remoteZone) &&
      visibleWithin(djLinkControl, disclosureStack) &&
      visibleWithin(djLinkControl, djLinkEndpointDesk),
    djLinkControlHitTestableAfterDisclosureScroll: hitTestableAtCenter(djLinkControl),
    djLinkActionControlReachability,
    djLinkActionControlsReachableAfterDisclosureScroll: djLinkActionControlReachability.every((result) => result.reachable),
    djLinkActionControlsHitTestableAfterDisclosureScroll: djLinkActionControlReachability.every((result) => result.hitTestable),
  };

  if (disclosureStack instanceof HTMLElement) disclosureStack.scrollTop = 0;
  if (remoteControl instanceof HTMLElement) remoteControl.scrollTop = 0;
  if (serverDesk instanceof HTMLElement) serverDesk.scrollTop = 0;
  for (const endpointDesk of remoteControl?.querySelectorAll('.remoteEndpointDesk') ?? []) {
    if (endpointDesk instanceof HTMLElement) endpointDesk.scrollTop = 0;
  }
  // Leave the primary Web Remote disclosure open for the caller's next
  // operation (the standard Setup I/O flow immediately starts the listener),
  // while returning every secondary peer disclosure to its closed baseline.
  for (const disclosure of disclosures) {
    if (disclosure instanceof HTMLDetailsElement) disclosure.open = disclosure === webRemoteDisclosure;
  }
  await settle();
  return {
    ...result,
    passed:
      result.remoteFound &&
      result.serverDeskFound &&
      result.serverHeaderFound &&
      result.remoteActionFound &&
      result.disclosureStackFound &&
      result.djLinkControlFound &&
      result.djLinkArmOrDisarmExclusive &&
      result.djLinkActionControlsFound &&
      result.allDisclosuresOpen &&
      result.remoteOwnsNoVerticalScroll &&
      result.disclosureStackOwnsVerticalScroll &&
        result.remoteScrollTopAfterDisclosureScroll === 0 &&
        result.disclosureStackScrollTopAfterDisclosureScroll > 0 &&
        result.remoteActionSizePreserved &&
        result.serverHeaderReachableAfterDisclosureScroll &&
        result.remoteActionReachableAfterDisclosureScroll &&
        result.remoteActionHitTestableAfterDisclosureScroll &&
        result.djLinkControlReachableAfterDisclosureScroll &&
        result.djLinkControlHitTestableAfterDisclosureScroll &&
        result.djLinkActionControlsReachableAfterDisclosureScroll &&
        result.djLinkActionControlsHitTestableAfterDisclosureScroll,
  };
}

export async function exerciseRemoteDisclosureScrollReachability(client, evaluatePageFunction) {
  return evaluatePageFunction(client, remoteDisclosureScrollInPage);
}
