import { Show } from "solid-js";
import type { SetupIoWorkspaceViewModel, FrontendViewInvoke } from "../App";
import {
channelFunctionDetail,
channelFunctionLabel,
channelFunctionRangeLabel
} from "../channelFunctionHelpers";
import { ArtRdmPanel } from "./ArtRdmPanel";
import { AudioOutputPanel } from "./AudioOutputPanel";
import { ChannelFunctionPanel } from "./ChannelFunctionPanel";
import { DmxInputPanel } from "./DmxInputPanel";
import { DmxOutputConfigPanel } from "./DmxOutputConfigPanel";
import { DmxRawMonitor } from "./DmxRawMonitor";
import { IoDisclosure,SetupIoConnectionDeck } from "./IoConnectionDeck";
import { LiveAudioInputRail } from "./LiveAudioInputRail";
import { LtcInputPanel } from "./LtcInputPanel";
import { MidiControlMappingPanel } from "./MidiControlMappingPanel";
import { MtcOutputSection } from "./MtcOutputSection";
import { OscControlMappingPanel } from "./OscControlMappingPanel";
import { OutputDiagnosticsPanel } from "./OutputDiagnosticsPanel";
import { RemoteControlPanel } from "./RemoteControlPanel";
import { ShowClockStatusPanel } from "./ShowClockStatusPanel";
import { StandbySyncPanel } from "./StandbySyncPanel";
import { controlMappingTargetLabel } from "../controlMappingLabels";
import {
outputProtocolLabel
} from "../createOutputDiagnosticsController";
import {
clampDmxValue
} from "../numericHelpers";

export function SetupIoWorkspaceView(props: { model: SetupIoWorkspaceViewModel; invokeCommand: FrontendViewInvoke }) {
  const invoke = props.invokeCommand;
  const {
    setupPanelClass,
    registerSetupPanel,
    activeIoConnection,
    setActiveIoConnection,
    output,
    audioOutputController,
    midiConnected,
    midiControlConnected,
    oscRunning,
    genericRemoteRunning,
    djListenerRunning,
    djLinkEnabled,
    extensionStatus,
    showClockStatus,
    serialPorts,
    serialDmxMachineBindingStatus,
    showSerialDmxSafetyBlackoutRouteStatus,
    snapshot,
    showDmxPreparationBusy,
    showDmxPreparationStage,
    prepareShowDmx,
    enableStagedShowArtNetLoopbackRoute,
    confirmSerialDmxMachineBinding,
    enableShowSerialDmxSafetyBlackoutRoute,
    stopShowSerialDmxSafetyBlackoutRoute,
    projectMappingsAuthority,
    applyDmxOutputRoutes,
    usbDmxUniverses,
    usbDmxUniversesStale,
    refreshUsbDmxUniverses,
    refreshSerialPorts,
    bindUsbDmxUniverse,
    clearUsbDmxUniverse,
    startUsbDmxUniverse,
    stopUsbDmxUniverse,
    dmxInputConfig,
    dmxInputStatus,
    dmxMappings,
    setDmxInputConfig,
    startDmxInput,
    stopDmxInput,
    setDmxMappings,
    sendArtRdmRequest,
    sendUsbRdmRequest,
    discoverUsbRdmDevices,
    discoverArtRdmDevices,
    startArtRdmFullDiscovery,
    dmxPreviewOptions,
    activeDmxPreviewUniverse,
    nonZeroDmxCount,
    dmxCells,
    setRawDmxUniverse,
    visibleFunctionControls,
    activeControlCategoryLabel,
    currentControlValue,
    channelFunctionContainsValue,
    channelFunctionBandStyle,
    channelFunctionSwatchColor,
    applyChannelFunction,
    dsf2026ArtNetAcceptanceProbeStatus,
    dmxTestChannel,
    dmxTestWidth,
    dmxTestValue,
    engineTelemetryReport,
    phase1SmokeReport,
    setDmxTestChannel,
    setDmxTestWidth,
    setDmxTestValue,
    sendDmxTestFrame,
    sendDmxRoutesTestFrame,
    sendDsf2026ArtNetAcceptanceProbe,
    acknowledgeDsf2026ArtNetAcceptanceProbeInDoubt,
    resetEngineTelemetry,
    saveEngineTelemetryReport,
    liveAudioInputRailProps,
    timelineCueAudioStatus,
    timelineCueAudioMutationBusy,
    timelineCueAudioLocalError,
    configureTimelineCueAudio,
    refreshTimelineCueAudioStatus,
    midiInputs,
    selectedMidiInput,
    midiOutputs,
    selectedMidiOutput,
    midiFeedbackConnected,
    midiFeedbackEnabled,
    midiFeedbackOutputStatus,
    midiMappings,
    midiMapMessage,
    midiMapNumber,
    midiMapChannel,
    midiMapAction,
    midiMapAttribute,
    midiClearFixtureFlagKind,
    selectedMidiCueId,
    selectedMidiEffectId,
    selectedMidiNodeGraphId,
    midiMapGroupId,
    selectedMidiLayerId,
    selectedMidiVideoOutputId,
    selectedMidiVideoOutputMappingField,
    selectedMidiVideoOutputMappingPresetLabel,
    midiMapVideoParam,
    midiMapCuePointIndex,
    midiMapDurationMs,
    midiMapLow,
    midiMapHigh,
    refreshMidiInputs,
    refreshMidiOutputs,
    setSelectedMidiInput,
    connectMidiClock,
    disconnectMidiClock,
    setSelectedMidiOutput,
    connectMidiFeedback,
    disconnectMidiFeedback,
    sendMidiFeedback,
    setMidiFeedbackEnabled,
    setMidiMapMessage,
    setMidiMapNumber,
    setMidiMapChannel,
    setMidiControlMappingAction,
    setMidiMapAttribute,
    setMidiMapCueId,
    setMidiMapEffectId,
    setMidiMapNodeGraphId,
    setMidiMapGroupId,
    setMidiMapLayerId,
    setMidiMapVideoOutputId,
    setMidiVideoOutputMappingFieldTarget,
    setMidiMapVideoOutputMappingPresetLabel,
    setMidiMapVideoParam,
    setMidiMapCuePointIndex,
    setMidiMapDurationMs,
    setMidiMapLow,
    setMidiMapHigh,
    controlLearnBusy,
    setWorkspaceTab,
    setControlLearnMode,
    learnMidiControl,
    addMidiMapping,
    connectMidiControl,
    disconnectMidiControl,
    loadMidiMappings,
    saveMidiMappings,
    removeMidiMapping,
    updateMidiMapping,
    isTauriRuntime,
    oscBindIp,
    oscPort,
    oscMappings,
    oscMapAddress,
    oscMapAction,
    oscMapAttribute,
    oscClearFixtureFlagKind,
    selectedOscCueId,
    selectedOscEffectId,
    selectedOscNodeGraphId,
    oscMapGroupId,
    selectedOscLayerId,
    selectedOscVideoOutputId,
    selectedOscVideoOutputMappingField,
    selectedOscVideoOutputMappingPresetLabel,
    oscMapVideoParam,
    oscMapCuePointIndex,
    oscMapDurationMs,
    oscMapLow,
    oscMapHigh,
    setOscBindIp,
    setOscPort,
    startOscInput,
    stopOscInput,
    setOscMapAddress,
    setOscControlMappingAction,
    setOscMapAttribute,
    setOscMapCueId,
    setOscMapEffectId,
    setOscMapNodeGraphId,
    setOscMapGroupId,
    setOscMapLayerId,
    setOscMapVideoOutputId,
    setOscVideoOutputMappingFieldTarget,
    setOscMapVideoOutputMappingPresetLabel,
    setOscMapVideoParam,
    setOscMapCuePointIndex,
    setOscMapDurationMs,
    setOscMapLow,
    setOscMapHigh,
    learnOscControl,
    addOscMapping,
    loadOscMappings,
    saveOscMappings,
    removeOscMapping,
    remoteBindIp,
    remotePort,
    remotePairingPin,
    remoteAllowLan,
    remoteMaxConnections,
    remoteMaxMessageBytes,
    remoteMaxMessagesPerSecond,
    djLinkMachineStatus,
    djLinkWiredCandidates,
    djLinkWiredCandidateCount,
    djLinkWiredRefreshBusy,
    djLinkWiredRefreshError,
    djLinkSelectedBinding,
    djLinkToken,
    djLinkTokenCopied,
    djTrackTriggers,
    djTimelineOptions,
    remoteRunning,
    remoteStatusHydrated,
    djLinkTokenOperationBusy,
    djLinkOperatorReturnBusy,
    remoteUrls,
    remoteStatus,
    setRemoteBindIp,
    setRemotePort,
    setRemotePairingPin,
    createPairingPin,
    setMessage,
    setRemoteAllowLan,
    setDjLinkBindIp,
    djLinkBindIp,
    setRemoteMaxConnections,
    setRemoteMaxMessageBytes,
    setRemoteMaxMessagesPerSecond,
    setDjLinkSelectedBinding,
    refreshDjLinkWiredCandidates,
    armDjLinkMachine,
    disarmDjLinkMachine,
    rotateDjLinkToken,
    copyDjLinkToken,
    requestDjLinkOperatorReturn,
    setDjTrackTriggersFromPanel,
    copyRemoteUrl,
    openRemoteUrl,
    startRemoteControl,
    stopRemoteControl,
    disconnectRemoteClient
  } = props.model;
  return (
    <aside
          class={setupPanelClass("panel output setupIoPanel setupPanel controlPanel", ["io"])}
          ref={registerSetupPanel(["io"])}
          tabIndex={-1}
          data-io-layout="operator"
        >
          <SetupIoConnectionDeck
            activeId={activeIoConnection()}
            onActiveId={setActiveIoConnection}
            outputEnabled={output().enabled}
            audioOutputSummary={audioOutputController.summary()}
            audioOutputState={audioOutputController.view().state}
            audioOutputStateTone={audioOutputController.stateTone()}
            midiClockConnected={midiConnected()}
            midiControlConnected={midiControlConnected()}
            oscRunning={oscRunning()}
            webRemoteRunning={genericRemoteRunning()}
            djListenerRunning={djListenerRunning()}
            djLinkEnabled={djLinkEnabled()}
            djLinkInstalled={extensionStatus().dj_link_enabled}
            syncRunning={showClockStatus()?.running ?? false}
            syncRole={showClockStatus()?.role ?? null}
            renderWorkbench={(connection) => connection === "dmx" ? (
          <section class="ioUnifiedZone ioUnifiedDmxZone" data-io-zone="dmx">
          <div class="ioOperatorSurface dmxOperatorSurface">
          <DmxOutputConfigPanel
            output={output()}
            serialPorts={serialPorts()}
            serialDmxMachineBindingStatus={serialDmxMachineBindingStatus()}
            showSerialDmxSafetyBlackoutRouteStatus={showSerialDmxSafetyBlackoutRouteStatus()}
            safetyBlackoutEngaged={snapshot().safety_blackout_engaged}
            showDmxPreparationBusy={showDmxPreparationBusy()}
            showDmxPreparationStage={showDmxPreparationStage()}
            onPrepareShowDmx={prepareShowDmx}
            onEnableStagedShowArtNetLoopbackRoute={enableStagedShowArtNetLoopbackRoute}
            onConfirmSerialDmxMachineBinding={confirmSerialDmxMachineBinding}
            onEnableShowSerialDmxSafetyBlackoutRoute={enableShowSerialDmxSafetyBlackoutRoute}
            onStopShowSerialDmxSafetyBlackoutRoute={stopShowSerialDmxSafetyBlackoutRoute}
            routes={snapshot().dmx_outputs.length > 0 ? snapshot().dmx_outputs : [snapshot().output]}
            routeTelemetry={snapshot().telemetry.last_dmx_route_results}
            projectEpoch={projectMappingsAuthority().project_epoch}
            onApplyNetworkRoutes={applyDmxOutputRoutes}
            usbDmxUniverses={usbDmxUniverses()}
            usbDmxUniversesStale={usbDmxUniversesStale()}
            onRefreshUsbDmxUniverses={refreshUsbDmxUniverses}
            onRescanSerialPorts={refreshSerialPorts}
            onBindUsbDmxUniverse={bindUsbDmxUniverse}
            onClearUsbDmxUniverse={clearUsbDmxUniverse}
            onStartUsbDmxUniverse={startUsbDmxUniverse}
            onStopUsbDmxUniverse={stopUsbDmxUniverse}
          />
          <div class="ioDisclosureStack">
          <IoDisclosure
            id="dmx-input"
            summary="DMX input and merge"
          >
          <DmxInputPanel
            config={dmxInputConfig()}
            status={dmxInputStatus()}
            mappings={dmxMappings()}
            onConfig={setDmxInputConfig}
            onStart={startDmxInput}
            onStop={stopDmxInput}
            onRemoveMapping={(index) => setDmxMappings((current) => current.filter((_, itemIndex) => itemIndex !== index))}
          />
          </IoDisclosure>
          <IoDisclosure
            id="dmx-rdm"
            summary="RDM tools"
            description="Use only with an RDM-capable gateway or USB interface to discover, inspect, or send RDM."
          >
          <ArtRdmPanel
            gatewayIp={output().target_ip}
            portAddress={output().universe}
            serialPorts={serialPorts()}
            onRequest={sendArtRdmRequest}
            onUsbRequest={sendUsbRdmRequest}
            onUsbDiscover={discoverUsbRdmDevices}
            onRefreshSerialPorts={refreshSerialPorts}
            onDiscover={discoverArtRdmDevices}
            onStartFullDiscovery={startArtRdmFullDiscovery}
          />
          </IoDisclosure>
          <IoDisclosure
            id="dmx-diagnostics"
            summary="DMX diagnostics and test"
            description="Test frames, raw monitor, telemetry, masters, and clock controls."
            bodyClass="dmxDiagnosticsDisclosure"
          >
          <DmxRawMonitor
            previews={dmxPreviewOptions()}
            activeUniverse={activeDmxPreviewUniverse()}
            activeCount={nonZeroDmxCount()}
            cells={dmxCells()}
            onUniverseChange={setRawDmxUniverse}
          >
            <Show when={visibleFunctionControls().length > 0}>
              <div class="dmxGdtfFunctionReadout">
                <ChannelFunctionPanel
                  categoryLabel={activeControlCategoryLabel()}
                  entries={visibleFunctionControls()}
                  currentValue={currentControlValue}
                  clampDmxValue={clampDmxValue}
                  functionContainsValue={channelFunctionContainsValue}
                  functionBandStyle={channelFunctionBandStyle}
                  functionLabel={channelFunctionLabel}
                  functionRangeLabel={channelFunctionRangeLabel}
                  functionDetail={channelFunctionDetail}
                  functionSwatchColor={channelFunctionSwatchColor}
                  onApplyFunction={(control, fn) => void applyChannelFunction(control, fn)}
                />
              </div>
            </Show>
          </DmxRawMonitor>
          <OutputDiagnosticsPanel
            output={output()}
            dsf2026ArtNetAcceptanceProbeStatus={dsf2026ArtNetAcceptanceProbeStatus}
            protocolLabel={outputProtocolLabel(output().protocol)}
            testChannel={dmxTestChannel()}
            testWidth={dmxTestWidth()}
            testValue={dmxTestValue()}
            telemetry={snapshot().telemetry}
            telemetryBudget={engineTelemetryReport()?.budget ?? null}
            phase1SmokeReport={phase1SmokeReport()}
            onTestChannel={setDmxTestChannel}
            onTestWidth={setDmxTestWidth}
            onTestValue={setDmxTestValue}
            onSendTest={sendDmxTestFrame}
            onSendRoutes={sendDmxRoutesTestFrame}
            onSendDsf2026ArtNetAcceptanceProbe={sendDsf2026ArtNetAcceptanceProbe}
            onAcknowledgeDsf2026ArtNetAcceptanceProbeInDoubt={acknowledgeDsf2026ArtNetAcceptanceProbeInDoubt}
            onResetTelemetry={resetEngineTelemetry}
            onSaveTelemetryReport={saveEngineTelemetryReport}
          />
          </IoDisclosure>
          </div>
           </div>
           </section>
             ) : connection === "audio" ? (
           <section class="ioUnifiedZone ioCompactZone" data-io-zone="audio">
           <div class="ioOperatorSurface audioOperatorSurface">
           <div
             id="setup-audio-input"
             class="setupAudioInput"
             role="region"
             aria-label="Audio input settings"
             tabIndex={-1}
           >
             <div class="setupAudioInputHeader">
               <strong>Audio Input</strong>
               <span>Live analysis</span>
             </div>
             <LiveAudioInputRail {...liveAudioInputRailProps} compact />
           </div>
           <AudioOutputPanel
             view={audioOutputController.view()}
             options={audioOutputController.options()}
             busy={audioOutputController.busy()}
             canRefresh={audioOutputController.canRefresh()}
             canRevalidate={audioOutputController.canRevalidate()}
             canStart={audioOutputController.canStart()}
             canStop={audioOutputController.canStop()}
             canReturnToNormal={audioOutputController.canReturnToNormal()}
             onBackendChange={audioOutputController.setBackend}
             onDriverChange={audioOutputController.setDriver}
             onSampleRateChange={audioOutputController.setSampleRate}
             onBufferFramesChange={audioOutputController.setBufferFrames}
             onProgramLeftChange={audioOutputController.setProgramLeft}
             onProgramRightChange={audioOutputController.setProgramRight}
             onCueChange={audioOutputController.setCue}
             onSpareChange={audioOutputController.setSpare}
             onRefresh={audioOutputController.refresh}
             onRevalidate={audioOutputController.revalidate}
             onStart={audioOutputController.start}
             onStop={audioOutputController.stop}
             onReturnToNormal={audioOutputController.returnToNormal}
             timelineCueAudioStatus={timelineCueAudioStatus()}
             timelineCueAudioMutationBusy={timelineCueAudioMutationBusy()}
             timelineCueAudioLocalError={timelineCueAudioLocalError()}
             onConfigureTimelineCueAudio={configureTimelineCueAudio}
             onRefreshTimelineCueAudio={async () => { await refreshTimelineCueAudioStatus(true, true); }}
           />
           </div>
           </section>
             ) : connection === "midi" ? (
          <section class="ioUnifiedZone ioCompactZone" data-io-zone="midi">
          <MidiControlMappingPanel
            snapshot={snapshot()}
            midiInputs={midiInputs()}
            selectedMidiInput={selectedMidiInput()}
            clockConnected={midiConnected()}
            midiOutputs={midiOutputs()}
            selectedMidiOutput={selectedMidiOutput()}
            feedbackConnected={midiFeedbackConnected()}
            feedbackEnabled={midiFeedbackEnabled()}
            feedbackOutputStatus={midiFeedbackOutputStatus()}
            midiInputsCount={midiInputs().length}
            controlConnected={midiControlConnected()}
            mappings={midiMappings()}
            mapMessage={midiMapMessage()}
            mapNumber={midiMapNumber()}
            mapChannel={midiMapChannel()}
            mapAction={midiMapAction()}
            mapAttribute={midiMapAttribute()}
            clearFixtureFlagKind={midiClearFixtureFlagKind()}
            selectedCueId={selectedMidiCueId()}
            selectedEffectId={selectedMidiEffectId()}
            selectedNodeGraphId={selectedMidiNodeGraphId()}
            mapGroupId={midiMapGroupId()}
            selectedLayerId={selectedMidiLayerId()}
            selectedOutputId={selectedMidiVideoOutputId()}
            selectedMappingField={selectedMidiVideoOutputMappingField()}
            selectedMappingPresetLabel={selectedMidiVideoOutputMappingPresetLabel()}
            mapVideoParam={midiMapVideoParam()}
            mapCuePointIndex={midiMapCuePointIndex()}
            mapDurationMs={midiMapDurationMs()}
            mapLow={midiMapLow()}
            mapHigh={midiMapHigh()}
            mappingTargetLabel={controlMappingTargetLabel}
            onRefreshMidi={() => {
              void refreshMidiInputs();
              void refreshMidiOutputs();
            }}
            onSelectedMidiInput={setSelectedMidiInput}
            onConnectClock={connectMidiClock}
            onDisconnectClock={disconnectMidiClock}
            onSelectedMidiOutput={setSelectedMidiOutput}
            onConnectFeedback={connectMidiFeedback}
            onDisconnectFeedback={disconnectMidiFeedback}
            onSendFeedback={sendMidiFeedback}
            onFeedbackEnabled={setMidiFeedbackEnabled}
            onMapMessage={setMidiMapMessage}
            onMapNumber={setMidiMapNumber}
            onMapChannel={setMidiMapChannel}
            onMapAction={setMidiControlMappingAction}
            onMapAttribute={setMidiMapAttribute}
            onMapCueId={setMidiMapCueId}
            onMapEffectId={setMidiMapEffectId}
            onMapNodeGraphId={setMidiMapNodeGraphId}
            onMapGroupId={setMidiMapGroupId}
            onMapLayerId={setMidiMapLayerId}
            onMapOutputId={setMidiMapVideoOutputId}
            onMappingField={setMidiVideoOutputMappingFieldTarget}
            onMappingPresetLabel={setMidiMapVideoOutputMappingPresetLabel}
            onMapVideoParam={setMidiMapVideoParam}
            onMapCuePointIndex={setMidiMapCuePointIndex}
            onMapDurationMs={setMidiMapDurationMs}
            onMapLow={setMidiMapLow}
            onMapHigh={setMidiMapHigh}
            learnBusy={controlLearnBusy()}
            onStartControlLearn={() => { setWorkspaceTab("touch"); setControlLearnMode("midi"); }}
            onLearn={learnMidiControl}
            onAddMapping={addMidiMapping}
            onConnectControl={async () => { await connectMidiControl(); }}
            onDisconnectControl={async () => { await disconnectMidiControl(); }}
            onLoadMappings={loadMidiMappings}
            onSaveMappings={saveMidiMappings}
            onRemoveMapping={removeMidiMapping}
            onUpdateMapping={updateMidiMapping}
          />
          <MtcOutputSection backendAvailable={isTauriRuntime()} invokeCommand={invoke} midiOutputs={midiOutputs()} />
          </section>
            ) : connection === "osc" ? (
          <section class="ioUnifiedZone ioCompactZone" data-io-zone="osc">
          <OscControlMappingPanel
            snapshot={snapshot()}
            bindIp={oscBindIp()}
            port={oscPort()}
            running={oscRunning()}
            mappings={oscMappings()}
            mapAddress={oscMapAddress()}
            mapAction={oscMapAction()}
            mapAttribute={oscMapAttribute()}
            clearFixtureFlagKind={oscClearFixtureFlagKind()}
            selectedCueId={selectedOscCueId()}
            selectedEffectId={selectedOscEffectId()}
            selectedNodeGraphId={selectedOscNodeGraphId()}
            mapGroupId={oscMapGroupId()}
            selectedLayerId={selectedOscLayerId()}
            selectedOutputId={selectedOscVideoOutputId()}
            selectedMappingField={selectedOscVideoOutputMappingField()}
            selectedMappingPresetLabel={selectedOscVideoOutputMappingPresetLabel()}
            mapVideoParam={oscMapVideoParam()}
            mapCuePointIndex={oscMapCuePointIndex()}
            mapDurationMs={oscMapDurationMs()}
            mapLow={oscMapLow()}
            mapHigh={oscMapHigh()}
            mappingTargetLabel={controlMappingTargetLabel}
            onBindIp={setOscBindIp}
            onPort={setOscPort}
            onStart={startOscInput}
            onStop={stopOscInput}
            onMapAddress={setOscMapAddress}
            onMapAction={setOscControlMappingAction}
            onMapAttribute={setOscMapAttribute}
            onMapCueId={setOscMapCueId}
            onMapEffectId={setOscMapEffectId}
            onMapNodeGraphId={setOscMapNodeGraphId}
            onMapGroupId={setOscMapGroupId}
            onMapLayerId={setOscMapLayerId}
            onMapOutputId={setOscMapVideoOutputId}
            onMappingField={setOscVideoOutputMappingFieldTarget}
            onMappingPresetLabel={setOscMapVideoOutputMappingPresetLabel}
            onMapVideoParam={setOscMapVideoParam}
            onMapCuePointIndex={setOscMapCuePointIndex}
            onMapDurationMs={setOscMapDurationMs}
            onMapLow={setOscMapLow}
            onMapHigh={setOscMapHigh}
            onLearn={learnOscControl}
            onAddMapping={addOscMapping}
            onLoadMappings={loadOscMappings}
            onSaveMappings={saveOscMappings}
            onRemoveMapping={removeOscMapping}
          />
          </section>
            ) : connection === "sync" ? (
          <section class="ioUnifiedZone ioCompactZone" data-io-zone="sync">
          <div class="ioOperatorSurface syncOperatorSurface">
            <StandbySyncPanel backendAvailable={isTauriRuntime()} invokeCommand={invoke} />
            <ShowClockStatusPanel backendAvailable={isTauriRuntime()} invokeCommand={invoke} />
            <LtcInputPanel backendAvailable={isTauriRuntime()} invokeCommand={invoke} />
          </div>
          </section>
            ) : (
          <section class="ioUnifiedZone ioCompactZone" data-io-zone="remote">
          <RemoteControlPanel
            // Keep the selected remote surface direct in the fixed I/O
            // workbench; its related safety, endpoint, standby, and DJ Link
            // sections remain grouped below as disclosures.
            surface={connection === "dj" ? "dj" : connection === "web" ? "web" : "all"}
            backendAvailable={isTauriRuntime()}
            invokeCommand={invoke}
            bindIp={remoteBindIp()}
            port={remotePort()}
            pairingPin={remotePairingPin()}
            allowLan={remoteAllowLan()}
            maxConnections={remoteMaxConnections()}
            maxMessageBytes={remoteMaxMessageBytes()}
            maxMessagesPerSecond={remoteMaxMessagesPerSecond()}
            djLinkEnabled={djLinkEnabled()}
            djLinkMachineStatus={djLinkMachineStatus()}
            djLinkWiredCandidates={djLinkWiredCandidates()}
            djLinkWiredCandidateCount={djLinkWiredCandidateCount()}
            djLinkWiredRefreshBusy={djLinkWiredRefreshBusy()}
            djLinkWiredRefreshError={djLinkWiredRefreshError()}
            djLinkSelectedBinding={djLinkSelectedBinding()}
            djLinkToken={djLinkToken()}
            djLinkTokenCopied={djLinkTokenCopied()}
            djTrackTriggers={djTrackTriggers()}
            timelineOptions={djTimelineOptions()}
            listenerRunning={remoteRunning()}
            listenerStatusHydrated={remoteStatusHydrated()}
            genericRunning={genericRemoteRunning()}
            djListenerRunning={djListenerRunning()}
            djLinkTokenOperationBusy={djLinkTokenOperationBusy()}
            djLinkOperatorReturnBusy={djLinkOperatorReturnBusy()}
            remoteUrls={remoteUrls()}
            status={remoteStatus()}
            onBindIp={setRemoteBindIp}
            onPort={setRemotePort}
            onPairingPin={setRemotePairingPin}
            onRegeneratePairingPin={() => {
              const nextPin = createPairingPin();
              if (!nextPin) {
                setMessage("Secure pairing PIN generation is unavailable; the existing PIN was kept.");
                return;
              }
              setRemotePairingPin(nextPin);
            }}
            onAllowLan={(value) => {
              if (!value && djLinkEnabled()) {
                setMessage("Disarm DJ Link before disabling trusted LAN access.");
                return;
              }
              setRemoteAllowLan(value);
              if (!value) {
                setDjLinkBindIp(null);
                setRemoteBindIp("127.0.0.1");
              } else {
                setRemoteBindIp(djLinkEnabled() ? (djLinkBindIp() ?? "0.0.0.0") : "0.0.0.0");
              }
            }}
            onMaxConnections={setRemoteMaxConnections}
            onMaxMessageBytes={setRemoteMaxMessageBytes}
            onMaxMessagesPerSecond={setRemoteMaxMessagesPerSecond}
            onDjLinkSelectedBinding={setDjLinkSelectedBinding}
            onRefreshDjLinkWiredCandidates={refreshDjLinkWiredCandidates}
            onArmDjLinkMachine={armDjLinkMachine}
            onDisarmDjLinkMachine={disarmDjLinkMachine}
            onRotateDjLinkToken={rotateDjLinkToken}
            onCopyDjLinkToken={copyDjLinkToken}
            onRequestDjLinkOperatorReturn={requestDjLinkOperatorReturn}
            onDjTrackTriggers={setDjTrackTriggersFromPanel}
            onCopyRemoteUrl={copyRemoteUrl}
            onOpenRemoteUrl={openRemoteUrl}
            onStart={startRemoteControl}
            onStop={stopRemoteControl}
            onDisconnectClient={disconnectRemoteClient}
          />
          </section>
            )}
          />
        </aside>
  );
}
