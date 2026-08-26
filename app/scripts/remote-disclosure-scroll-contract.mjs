async function remoteDisclosureScrollInPage() {
  const remoteZone = document.querySelector('[data-io-zone="remote"]');
  const remoteControl = remoteZone?.querySelector('.remoteControl');
  const serverDesk = remoteControl?.querySelector(':scope > .remoteServerDesk');
  const serverHeader = serverDesk?.querySelector(':scope > .ioDeskHeader');
  const remoteAction = serverDesk?.querySelector('[data-io-control="remote-start"], [data-io-control="remote-stop"]');
  const disclosureStack = remoteControl?.querySelector(':scope > .ioDisclosureStack');
  const disclosureNames = ['remote-security', 'remote-endpoints', 'dj-link', 'remote-standby'];
  const disclosures = disclosureNames.map((name) =>
    remoteControl?.querySelector(`[data-io-disclosure="${name}"]`) ?? null,
  );
  const djLinkControl = remoteControl?.querySelector('[data-io-control="dj-link-enabled"]');
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

  const remoteActionSizeBeforeDisclosureScroll = sizeOf(remoteAction);
  if (remoteControl instanceof HTMLElement) remoteControl.scrollTop = 0;
  if (disclosureStack instanceof HTMLElement) disclosureStack.scrollTop = 0;
  for (const disclosure of disclosures) {
    const summary = disclosure?.querySelector(':scope > summary');
    if (summary instanceof HTMLElement && !disclosure.open) summary.click();
  }
  await settle();

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
  djLinkControl?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  await settle();
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
    allDisclosuresOpen,
    remoteOwnsNoVerticalScroll: remoteStyle?.overflowY === 'hidden',
    disclosureStackOwnsVerticalScroll: disclosureStackStyle?.overflowY === 'auto' && disclosureStackScrollable,
    remoteScrollTopAfterDisclosureScroll: remoteControl instanceof HTMLElement ? remoteControl.scrollTop : -1,
    disclosureStackScrollTopAfterDisclosureScroll:
      disclosureStack instanceof HTMLElement ? disclosureStack.scrollTop : -1,
    remoteActionSizeBeforeDisclosureScroll,
    remoteActionSizeAfterDisclosureScroll,
    remoteActionSizePreserved,
    serverHeaderReachableAfterDisclosureScroll: visibleWithin(serverHeader, remoteZone),
    remoteActionReachableAfterDisclosureScroll: visibleWithin(remoteAction, remoteZone),
    remoteActionHitTestableAfterDisclosureScroll: hitTestableAtCenter(remoteAction),
    djLinkControlReachableAfterDisclosureScroll: visibleWithin(djLinkControl, remoteZone),
  };

  if (disclosureStack instanceof HTMLElement) disclosureStack.scrollTop = 0;
  if (remoteControl instanceof HTMLElement) remoteControl.scrollTop = 0;
  for (const disclosure of disclosures) {
    if (disclosure instanceof HTMLDetailsElement) disclosure.open = false;
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
      result.allDisclosuresOpen &&
      result.remoteOwnsNoVerticalScroll &&
      result.disclosureStackOwnsVerticalScroll &&
        result.remoteScrollTopAfterDisclosureScroll === 0 &&
        result.disclosureStackScrollTopAfterDisclosureScroll > 0 &&
        result.remoteActionSizePreserved &&
        result.serverHeaderReachableAfterDisclosureScroll &&
        result.remoteActionReachableAfterDisclosureScroll &&
        result.remoteActionHitTestableAfterDisclosureScroll &&
        result.djLinkControlReachableAfterDisclosureScroll,
  };
}

export async function exerciseRemoteDisclosureScrollReachability(client, evaluatePageFunction) {
  return evaluatePageFunction(client, remoteDisclosureScrollInPage);
}
