import { serialDmxDeviceInstance } from "./serialDmxDeviceInstance";
import type { SerialDmxMachineBindingIdentity, SerialPortSummary } from "./types";

const hasNonemptyUsbIdentityText = (value: string | null | undefined) =>
  typeof value === "string" && value.trim().length > 0;

/**
 * This UI eligibility is deliberately as strict as the native machine-local
 * binding boundary. A visible COM/PnP row with a missing or zero VID/PID is
 * diagnostic data only: it must never become a Confirm candidate.
 */
export const hasExactMachineLocalOpenDmxIdentity = (
  port: SerialPortSummary | undefined,
): port is SerialPortSummary => Boolean(
  port
  && hasNonemptyUsbIdentityText(port.name)
  && hasNonemptyUsbIdentityText(port.port_type)
  && typeof port.usb_vid === "number"
  && Number.isInteger(port.usb_vid)
  && port.usb_vid > 0
  && port.usb_vid <= 0xffff
  && typeof port.usb_pid === "number"
  && Number.isInteger(port.usb_pid)
  && port.usb_pid > 0
  && port.usb_pid <= 0xffff
  && hasNonemptyUsbIdentityText(port.serial_number)
  && hasNonemptyUsbIdentityText(port.manufacturer)
  && hasNonemptyUsbIdentityText(port.product)
  && Boolean(serialDmxDeviceInstance(port)),
);

/**
 * The same physical cable, by any stable identity. Mirrors the native rule:
 * COM name, platform instance, or VID+PID+serial (a cloned serial counts as
 * the same cable so a port swap can never swap universes).
 */
export const sameUsbDmxCable = (
  identity: SerialDmxMachineBindingIdentity,
  port: SerialPortSummary,
): boolean => {
  const sameText = (left: string | null | undefined, right: string | null | undefined) =>
    Boolean(left && right && left.toLowerCase() === right.toLowerCase());
  return sameText(identity.port_name, port.name)
    || sameText(identity.windows_device_instance_id, port.windows_device_instance_id)
    || sameText(identity.macos_device_instance_id, port.macos_device_instance_id)
    || (identity.usb_vid === port.usb_vid
      && identity.usb_pid === port.usb_pid
      && identity.serial_number === port.serial_number);
};
