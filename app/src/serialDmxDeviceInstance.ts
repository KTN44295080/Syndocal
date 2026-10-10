import type { SerialPortSummary } from "./types";

// A COM or BSD port name is never sufficient hardware identity.
export function serialDmxDeviceInstance(port: Pick<SerialPortSummary, "name" | "windows_device_instance_id" | "macos_device_instance_id">): string | null {
  const windows = port.windows_device_instance_id;
  const macos = port.macos_device_instance_id;
  if (windows != null && macos == null) return windows.trim() ? windows : null;
  if (windows == null && macos != null && (port.name.startsWith("/dev/cu.") || port.name === `usb-ftdi://${macos.slice(6)}`)
      && /^ioreg:[0-9a-f]{16}$/i.test(macos) && !/^ioreg:0{16}$/.test(macos)) return macos;
  return null;
}

export function serialDmxConfirmationRequest(port: SerialPortSummary) {
  if (!serialDmxDeviceInstance(port)) {
    throw new Error("USB-DMX confirmation requires the current device instance; refresh serial devices and select again.");
  }
  return {
    portName: port.name,
    windowsDeviceInstanceId: port.windows_device_instance_id ?? null,
    macosDeviceInstanceId: port.macos_device_instance_id ?? null,
  };
}
