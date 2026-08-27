async function remoteDisclosureScrollInPage() {
  const settle = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const visible = (element) => {
    if (!(element instanceof HTMLElement)) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  };
  const sizeOf = (element) => {
    if (!element) return null;
    const rect = element.getBoundingClientRect();
    return { width: rect.width, height: rect.height };
  };
  const visibleWithin = (element, container) => {
    if (!visible(element) || !visible(container)) return false;
    const elementRect = element.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    return (
      elementRect.left >= Math.max(0, containerRect.left) - 1 &&
      elementRect.right <= Math.min(innerWidth, containerRect.right) + 1 &&
      elementRect.top >= Math.max(0, containerRect.top) - 1 &&
      elementRect.bottom <= Math.min(innerHeight, containerRect.bottom) + 1
    );
  };
  const hitTestableAtCenter = (element) => {
    if (!visible(element)) return false;
    const rect = element.getBoundingClientRect();
    const target = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
    return target === element || element.contains(target);
  };
  const openDisclosure = (disclosure) => {
    const summary = disclosure?.querySelector(':scope > summary');
    if (summary instanceof HTMLElement && !disclosure.open) summary.click();
  };
  const closeDisclosure = (disclosure) => {
    if (disclosure instanceof HTMLDetailsElement) disclosure.open = false;
  };
  const selectConnection = async (id) => {
    const card = document.querySelector(`[data-io-primary-connection="${id}"]`);
    const button = card?.querySelector(':scope > button[aria-controls]');
    if (!(button instanceof HTMLElement)) return { id, found: false, selected: false };
    if (button.getAttribute('aria-pressed') !== 'true') button.click();
    await settle();
    const body = document.querySelector('[data-io-workbench-body]');
    const zone = body?.querySelector('[data-io-zone]');
    const disclosureStack = zone?.querySelector('.remoteControl > .ioDisclosureStack');
    const disclosureMounted = (name) => Boolean(
      disclosureStack?.querySelector(`:scope > [data-io-disclosure="${name}"]`),
    );
    return {
      id,
      found: true,
      selected: button.getAttribute('aria-pressed') === 'true',
      zone: zone?.getAttribute('data-io-zone') ?? null,
      zoneMountedInWorkbench: Boolean(zone && body?.contains(zone)),
      mountedDisclosures: {
        webRemote: disclosureMounted('web-remote'),
        remoteSecurity: disclosureMounted('remote-security'),
        remoteEndpoints: disclosureMounted('remote-endpoints'),
        remoteStandby: disclosureMounted('remote-standby'),
        djLink: disclosureMounted('dj-link'),
      },
    };
  };
  const resetScroll = (root) => {
    if (!(root instanceof HTMLElement)) return;
    root.scrollTop = 0;
    root.scrollLeft = 0;
    for (const element of root.querySelectorAll('*')) {
      if (element instanceof HTMLElement) {
        element.scrollTop = 0;
        element.scrollLeft = 0;
      }
    }
  };
  const reveal = async (element, root) => {
    if (!(element instanceof HTMLElement)) return;
    const scrollables = [];
    for (let ancestor = element.parentElement; ancestor; ancestor = ancestor.parentElement) {
      const style = getComputedStyle(ancestor);
      if (/(auto|scroll)/.test(style.overflowY) || /(auto|scroll)/.test(style.overflowX)) {
        scrollables.push(ancestor);
      }
      if (ancestor === root) break;
    }
    for (const scrollport of scrollables) {
      const target = element.getBoundingClientRect();
      const container = scrollport.getBoundingClientRect();
      if (target.top < container.top) scrollport.scrollTop += target.top - container.top - 2;
      if (target.bottom > container.bottom) scrollport.scrollTop += target.bottom - container.bottom + 2;
      if (target.left < container.left) scrollport.scrollLeft += target.left - container.left - 2;
      if (target.right > container.right) scrollport.scrollLeft += target.right - container.right + 2;
      await settle();
    }
  };
  const readWebPath = async () => {
    const remoteZone = document.querySelector('[data-io-zone="remote"]');
    const workbenchBody = remoteZone?.closest('[data-io-workbench-body]');
    const remoteControl = remoteZone?.querySelector('.remoteControl');
    const disclosureStack = remoteControl?.querySelector(':scope > .ioDisclosureStack');
    const webRemoteDisclosure = disclosureStack?.querySelector(':scope > [data-io-disclosure="web-remote"]') ?? null;
    const remoteSecurityDisclosure = disclosureStack?.querySelector(':scope > [data-io-disclosure="remote-security"]') ?? null;
    const remoteEndpointsDisclosure = disclosureStack?.querySelector(':scope > [data-io-disclosure="remote-endpoints"]') ?? null;
    const remoteStandbyDisclosure = disclosureStack?.querySelector(':scope > [data-io-disclosure="remote-standby"]') ?? null;
    const webDisclosures = [webRemoteDisclosure, remoteSecurityDisclosure, remoteEndpointsDisclosure, remoteStandbyDisclosure];
    const webRemoteBody = webRemoteDisclosure?.querySelector(':scope > [data-io-disclosure-body]') ?? null;
    const serverDesk = webRemoteBody?.querySelector(':scope > .remoteServerDesk');
    const serverHeader = serverDesk?.querySelector(':scope > .ioDeskHeader');
    const remoteAction = serverDesk?.querySelector('[data-io-control="remote-start"], [data-io-control="remote-stop"]');
    if (workbenchBody instanceof HTMLElement) resetScroll(workbenchBody);
    if (remoteControl instanceof HTMLElement) resetScroll(remoteControl);
    if (disclosureStack instanceof HTMLElement) resetScroll(disclosureStack);
    for (const disclosure of webDisclosures) openDisclosure(disclosure);
    for (const nested of webRemoteDisclosure?.querySelectorAll('details') ?? []) openDisclosure(nested);
    await settle();
    const remoteActionSizeBeforeDisclosureScroll = sizeOf(remoteAction);
    const remoteStyle = remoteControl ? getComputedStyle(remoteControl) : null;
    const workbenchStyle = workbenchBody ? getComputedStyle(workbenchBody) : null;
    const workbenchScrollable = Boolean(
      workbenchBody instanceof HTMLElement && workbenchBody.scrollHeight > workbenchBody.clientHeight,
    );
    if (workbenchBody instanceof HTMLElement) workbenchBody.scrollTop = workbenchBody.scrollHeight;
    await settle();
    await reveal(serverHeader, workbenchBody);
    const serverHeaderReachable = visibleWithin(serverHeader, workbenchBody);
    await reveal(remoteAction, workbenchBody);
    const remoteActionReachable = visibleWithin(remoteAction, workbenchBody);
    const remoteActionHitTestable = hitTestableAtCenter(remoteAction);
    const remoteActionSizeAfterDisclosureScroll = sizeOf(remoteAction);
    const remoteActionSizePreserved = Boolean(
      remoteActionSizeBeforeDisclosureScroll && remoteActionSizeAfterDisclosureScroll &&
      Math.abs(remoteActionSizeBeforeDisclosureScroll.width - remoteActionSizeAfterDisclosureScroll.width) <= 0.5 &&
      Math.abs(remoteActionSizeBeforeDisclosureScroll.height - remoteActionSizeAfterDisclosureScroll.height) <= 0.5,
    );
    return {
      remoteFound: remoteControl instanceof HTMLElement,
      serverDeskFound: serverDesk instanceof HTMLElement,
      serverHeaderFound: serverHeader instanceof HTMLElement,
      remoteActionFound: remoteAction instanceof HTMLElement,
      disclosureStackFound: disclosureStack instanceof HTMLElement,
      workbenchBodyFound: workbenchBody instanceof HTMLElement,
      disclosureCount: webDisclosures.length,
      allWebDisclosuresOpen: webDisclosures.every((disclosure) => disclosure?.open === true),
      remoteOwnsNoVerticalScroll: remoteStyle?.overflowY === 'hidden',
      workbenchOwnsVerticalScroll: workbenchStyle?.overflowY === 'auto',
      workbenchScrollable,
      remoteScrollTopAfterDisclosureScroll: remoteControl instanceof HTMLElement ? remoteControl.scrollTop : -1,
      workbenchScrollTopAfterDisclosureScroll: workbenchBody instanceof HTMLElement ? workbenchBody.scrollTop : -1,
      remoteActionSizeBeforeDisclosureScroll,
      remoteActionSizeAfterDisclosureScroll,
      remoteActionSizePreserved,
      serverHeaderReachableAfterDisclosureScroll: serverHeaderReachable,
      remoteActionReachableAfterDisclosureScroll: remoteActionReachable,
      remoteActionHitTestableAfterDisclosureScroll: remoteActionHitTestable,
      passed:
        remoteControl instanceof HTMLElement &&
        serverDesk instanceof HTMLElement &&
        serverHeader instanceof HTMLElement &&
        remoteAction instanceof HTMLElement &&
        disclosureStack instanceof HTMLElement &&
        workbenchBody instanceof HTMLElement &&
        webDisclosures.every((disclosure) => disclosure?.open === true) &&
        remoteStyle?.overflowY === 'hidden' &&
        workbenchStyle?.overflowY === 'auto' &&
        remoteControl.scrollTop === 0 &&
        remoteActionSizePreserved &&
        serverHeaderReachable &&
        remoteActionReachable &&
        remoteActionHitTestable,
    };
  };
  const readDjPath = async () => {
    const remoteZone = document.querySelector('[data-io-zone="remote"]');
    const workbenchBody = remoteZone?.closest('[data-io-workbench-body]');
    const remoteControl = remoteZone?.querySelector('.remoteControl');
    const disclosureStack = remoteControl?.querySelector(':scope > .ioDisclosureStack');
    const djDisclosure = disclosureStack?.querySelector(':scope > [data-io-disclosure="dj-link"]') ?? null;
    const djLinkBody = djDisclosure?.querySelector(':scope > [data-io-disclosure-body]') ?? null;
    const djLinkControl = djLinkBody?.querySelector('[data-io-control="dj-link-wired-binding"]');
    const djLinkRefreshControl = djLinkBody?.querySelector('[data-io-control="dj-link-refresh-wired-candidates"]');
    const djLinkArmControl = djLinkBody?.querySelector('[data-io-control="dj-link-arm"]');
    const djLinkDisarmControl = djLinkBody?.querySelector('[data-io-control="dj-link-disarm"]');
    const djLinkRotateControl = djLinkBody?.querySelector('[data-io-control="dj-link-rotate-token"]');
    const djLinkArmOrDisarmControl = djLinkArmControl ?? djLinkDisarmControl;
    const actionControls = [djLinkRefreshControl, djLinkArmOrDisarmControl, djLinkRotateControl];
    if (workbenchBody instanceof HTMLElement) resetScroll(workbenchBody);
    if (remoteControl instanceof HTMLElement) resetScroll(remoteControl);
    if (disclosureStack instanceof HTMLElement) resetScroll(disclosureStack);
    openDisclosure(djDisclosure);
    await settle();
    if (workbenchBody instanceof HTMLElement) workbenchBody.scrollTop = workbenchBody.scrollHeight;
    await settle();
    for (const control of [djLinkControl, ...actionControls]) await reveal(control, workbenchBody);
    const reachability = actionControls.map((control) => ({
      control: control?.getAttribute('data-io-control') ?? null,
      reachable: visibleWithin(control, workbenchBody),
      hitTestable: hitTestableAtCenter(control),
    }));
    await reveal(djLinkControl, workbenchBody);
    const remoteStyle = remoteControl ? getComputedStyle(remoteControl) : null;
    const workbenchStyle = workbenchBody ? getComputedStyle(workbenchBody) : null;
    const workbenchScrollable = Boolean(
      workbenchBody instanceof HTMLElement && workbenchBody.scrollHeight > workbenchBody.clientHeight,
    );
    return {
      remoteFound: remoteControl instanceof HTMLElement,
      disclosureStackFound: disclosureStack instanceof HTMLElement,
      workbenchBodyFound: workbenchBody instanceof HTMLElement,
      djLinkDisclosureFound: djDisclosure instanceof HTMLElement,
      djLinkControlFound: djLinkControl instanceof HTMLElement,
      djLinkArmControlFound: djLinkArmControl instanceof HTMLElement,
      djLinkDisarmControlFound: djLinkDisarmControl instanceof HTMLElement,
      djLinkArmOrDisarmExclusive: (djLinkArmControl instanceof HTMLElement) !== (djLinkDisarmControl instanceof HTMLElement),
      djLinkActionControlsFound: actionControls.every((control) => control instanceof HTMLElement),
      disclosureOpen: djDisclosure?.open === true,
      remoteOwnsNoVerticalScroll: remoteStyle?.overflowY === 'hidden',
      workbenchOwnsVerticalScroll: workbenchStyle?.overflowY === 'auto',
      workbenchScrollable,
      remoteScrollTopAfterDisclosureScroll: remoteControl instanceof HTMLElement ? remoteControl.scrollTop : -1,
      workbenchScrollTopAfterDisclosureScroll: workbenchBody instanceof HTMLElement ? workbenchBody.scrollTop : -1,
      djLinkControlReachableAfterDisclosureScroll:
        visibleWithin(djLinkControl, workbenchBody),
      djLinkControlHitTestableAfterDisclosureScroll: hitTestableAtCenter(djLinkControl),
      djLinkActionControlReachability: reachability,
      djLinkActionControlsReachableAfterDisclosureScroll: reachability.every((result) => result.reachable),
      djLinkActionControlsHitTestableAfterDisclosureScroll: reachability.every((result) => result.hitTestable),
      passed:
        remoteControl instanceof HTMLElement &&
        disclosureStack instanceof HTMLElement &&
        workbenchBody instanceof HTMLElement &&
        djDisclosure instanceof HTMLElement &&
        djLinkControl instanceof HTMLElement &&
        djLinkArmOrDisarmControl instanceof HTMLElement &&
        djLinkRotateControl instanceof HTMLElement &&
        djLinkRefreshControl instanceof HTMLElement &&
        djDisclosure.open &&
        remoteStyle?.overflowY === 'hidden' &&
        workbenchStyle?.overflowY === 'auto' &&
        remoteControl.scrollTop === 0 &&
        visibleWithin(djLinkControl, workbenchBody) &&
        hitTestableAtCenter(djLinkControl) &&
        reachability.every((result) => result.reachable && result.hitTestable),
    };
  };

  const webSelection = await selectConnection('web');
  const web = await readWebPath();
  const djSelection = await selectConnection('dj');
  const dj = await readDjPath();
  const restoredWebSelection = await selectConnection('web');
  const restoredWebZone = document.querySelector('[data-io-zone="remote"]');
  const restoredWebStack = restoredWebZone?.querySelector('.remoteControl > .ioDisclosureStack');
  const restoredWebDisclosure = restoredWebStack?.querySelector('[data-io-disclosure="web-remote"]');
  if (restoredWebDisclosure instanceof HTMLDetailsElement) {
    openDisclosure(restoredWebDisclosure);
    for (const disclosure of restoredWebStack?.querySelectorAll('[data-io-disclosure]') ?? []) {
      if (disclosure !== restoredWebDisclosure) closeDisclosure(disclosure);
    }
  }
  await settle();
  return {
    webSelection,
    djSelection,
    restoredWebSelection,
    web,
    dj,
    webCardReachable: webSelection.found && webSelection.selected && webSelection.zone === 'remote',
    djCardReachable: djSelection.found && djSelection.selected && djSelection.zone === 'remote',
    passed:
      webSelection.found && webSelection.selected && webSelection.zoneMountedInWorkbench &&
      webSelection.mountedDisclosures.webRemote &&
      webSelection.mountedDisclosures.remoteSecurity &&
      webSelection.mountedDisclosures.remoteEndpoints &&
      webSelection.mountedDisclosures.remoteStandby &&
      !webSelection.mountedDisclosures.djLink &&
      djSelection.found && djSelection.selected && djSelection.zoneMountedInWorkbench &&
      djSelection.mountedDisclosures.djLink &&
      !djSelection.mountedDisclosures.webRemote &&
      !djSelection.mountedDisclosures.remoteSecurity &&
      !djSelection.mountedDisclosures.remoteEndpoints &&
      !djSelection.mountedDisclosures.remoteStandby &&
      restoredWebSelection.found && restoredWebSelection.selected &&
      restoredWebSelection.mountedDisclosures.webRemote &&
      restoredWebSelection.mountedDisclosures.remoteStandby &&
      !restoredWebSelection.mountedDisclosures.djLink &&
      web.passed && dj.passed,
  };
}

export async function exerciseRemoteDisclosureScrollReachability(client, evaluatePageFunction) {
  return evaluatePageFunction(client, remoteDisclosureScrollInPage);
}
