/* FT232R output through Apple's USB user-client API. No driver installation,
 * reset of unrelated devices, automatic kernel detach, or unbounded I/O. */
#include <CoreFoundation/CoreFoundation.h>
#include <IOKit/IOKitLib.h>
#include <IOKit/IOCFPlugIn.h>
#include <IOKit/usb/IOUSBLib.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    IOUSBDeviceInterface320 **device;
    IOUSBInterfaceInterface550 **interface;
    UInt8 output_pipe;
    int device_open;
    int interface_open;
} SyndocalFtdi;

void syndocal_ftdi_close(SyndocalFtdi *port) {
    if (!port) return;
    if (port->interface) {
        if (port->interface_open) (*port->interface)->USBInterfaceClose(port->interface);
        (*port->interface)->Release(port->interface);
    }
    if (port->device) {
        if (port->device_open) (*port->device)->USBDeviceClose(port->device);
        (*port->device)->Release(port->device);
    }
    free(port);
}

SyndocalFtdi *syndocal_ftdi_open(uint64_t registry_id, int32_t *status) {
    io_service_t service = IOServiceGetMatchingService(0, IORegistryEntryIDMatching(registry_id));
    if (!service) { *status = kIOReturnNoDevice; return NULL; }
    SyndocalFtdi *port = calloc(1, sizeof(*port));
    if (!port) { IOObjectRelease(service); *status = kIOReturnNoMemory; return NULL; }
    IOCFPlugInInterface **plugin = NULL;
    SInt32 score = 0;
    *status = IOCreatePlugInInterfaceForService(service, kIOUSBDeviceUserClientTypeID,
                                               kIOCFPlugInInterfaceID, &plugin, &score);
    IOObjectRelease(service);
    if (*status != kIOReturnSuccess || !plugin) goto fail;
    HRESULT queried = (*plugin)->QueryInterface(plugin, CFUUIDGetUUIDBytes(kIOUSBDeviceInterfaceID320), (LPVOID *)&port->device);
    (*plugin)->Release(plugin); plugin = NULL;
    if (queried != 0 || !port->device) { *status = kIOReturnUnsupported; goto fail; }
    UInt16 vendor = 0, product = 0;
    *status = (*port->device)->GetDeviceVendor(port->device, &vendor);
    if (*status != kIOReturnSuccess) goto fail;
    *status = (*port->device)->GetDeviceProduct(port->device, &product);
    if (*status != kIOReturnSuccess) goto fail;
    if (vendor != 0x0403 || product != 0x6001) { *status = kIOReturnUnsupported; goto fail; }
    /* Never seize another application's or a kernel driver's handle. */
    *status = (*port->device)->USBDeviceOpen(port->device);
    if (*status != kIOReturnSuccess) goto fail;
    port->device_open = 1;
    UInt8 configuration = 0;
    *status = (*port->device)->GetConfiguration(port->device, &configuration);
    if (*status != kIOReturnSuccess) goto fail;
    if (configuration == 0) {
        IOUSBConfigurationDescriptorPtr descriptor = NULL;
        *status = (*port->device)->GetConfigurationDescriptorPtr(port->device, 0, &descriptor);
        if (*status != kIOReturnSuccess || !descriptor) goto fail;
        *status = (*port->device)->SetConfiguration(port->device, descriptor->bConfigurationValue);
        if (*status != kIOReturnSuccess) goto fail;
    }
    IOUSBFindInterfaceRequest request = { kIOUSBFindInterfaceDontCare, kIOUSBFindInterfaceDontCare,
                                         kIOUSBFindInterfaceDontCare, kIOUSBFindInterfaceDontCare };
    io_iterator_t iterator = 0;
    *status = (*port->device)->CreateInterfaceIterator(port->device, &request, &iterator);
    if (*status != kIOReturnSuccess) goto fail;
    io_service_t interface_service = IOIteratorNext(iterator);
    io_service_t extra = IOIteratorNext(iterator);
    IOObjectRelease(iterator);
    if (!interface_service || extra) {
        if (interface_service) IOObjectRelease(interface_service);
        if (extra) IOObjectRelease(extra);
        *status = kIOReturnUnsupported; goto fail;
    }
    *status = IOCreatePlugInInterfaceForService(interface_service, kIOUSBInterfaceUserClientTypeID,
                                               kIOCFPlugInInterfaceID, &plugin, &score);
    IOObjectRelease(interface_service);
    if (*status != kIOReturnSuccess || !plugin) goto fail;
    queried = (*plugin)->QueryInterface(plugin, CFUUIDGetUUIDBytes(kIOUSBInterfaceInterfaceID550), (LPVOID *)&port->interface);
    (*plugin)->Release(plugin); plugin = NULL;
    if (queried != 0 || !port->interface) { *status = kIOReturnUnsupported; goto fail; }
    *status = (*port->interface)->USBInterfaceOpen(port->interface);
    if (*status != kIOReturnSuccess) goto fail;
    port->interface_open = 1;
    UInt8 endpoints = 0;
    *status = (*port->interface)->GetNumEndpoints(port->interface, &endpoints);
    if (*status != kIOReturnSuccess) goto fail;
    for (UInt8 pipe = 1; pipe <= endpoints; pipe++) {
        UInt8 direction = 0, number = 0, type = 0, interval = 0;
        UInt16 max_packet = 0;
        *status = (*port->interface)->GetPipeProperties(port->interface, pipe,
            &direction, &number, &type, &max_packet, &interval);
        if (*status != kIOReturnSuccess) goto fail;
        if (direction == kUSBOut && type == kUSBBulk) {
            if (port->output_pipe) { *status = kIOReturnUnsupported; goto fail; }
            port->output_pipe = pipe;
        }
    }
    if (!port->output_pipe) { *status = kIOReturnUnsupported; goto fail; }
    *status = kIOReturnSuccess;
    return port;
fail:
    if (plugin) (*plugin)->Release(plugin);
    syndocal_ftdi_close(port);
    return NULL;
}

int32_t syndocal_ftdi_control(SyndocalFtdi *port, uint8_t direction, uint8_t request,
                              uint16_t value, uint16_t index, void *data, uint16_t size,
                              uint32_t timeout_ms) {
    IOUSBDevRequestTO transfer;
    memset(&transfer, 0, sizeof(transfer));
    transfer.bmRequestType = direction; transfer.bRequest = request;
    transfer.wValue = value; transfer.wIndex = index;
    transfer.wLength = size; transfer.pData = data;
    transfer.noDataTimeout = timeout_ms; transfer.completionTimeout = timeout_ms;
    IOReturn status = (*port->device)->DeviceRequestTO(port->device, &transfer);
    if (status == kIOReturnSuccess && transfer.wLenDone != size) return kIOReturnUnderrun;
    return status;
}

int32_t syndocal_ftdi_write(SyndocalFtdi *port, const void *data, uint32_t size, uint32_t timeout_ms) {
    return (*port->interface)->WritePipeTO(port->interface, port->output_pipe,
        (void *)data, size, timeout_ms, timeout_ms);
}
