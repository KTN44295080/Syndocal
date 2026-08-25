#ifndef SYNDOCAL_ASIO_BRIDGE_H
#define SYNDOCAL_ASIO_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef _WIN32
#define SYNDOCAL_ASIO_CALL __cdecl
#else
#define SYNDOCAL_ASIO_CALL
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define SYNDOCAL_ASIO_ABI_VERSION 2u
#define SYNDOCAL_ASIO_JSON_SCHEMA_VERSION 2u
#define SYNDOCAL_ASIO_CANONICAL_DLL "syndocal_asio_bridge.dll"
#define SYNDOCAL_ASIO_BUILD_FLAG_ASIO_COMPILED 0x1u

typedef struct SyndocalAsioHandle SyndocalAsioHandle;

typedef struct SyndocalAsioStringV2 {
    uint8_t *ptr;
    size_t len;
} SyndocalAsioStringV2;

enum SyndocalAsioStatusV2 {
    SYNDOCAL_ASIO_V2_OK = 0,
    SYNDOCAL_ASIO_V2_INVALID_ARGUMENT = 1,
    SYNDOCAL_ASIO_V2_UNSUPPORTED = 2,
    SYNDOCAL_ASIO_V2_DRIVER_NOT_FOUND = 3,
    SYNDOCAL_ASIO_V2_CONFIG_UNSUPPORTED = 4,
    SYNDOCAL_ASIO_V2_BACKEND_ERROR = 5,
    SYNDOCAL_ASIO_V2_TERMINAL = 6,
    SYNDOCAL_ASIO_V2_PANIC = 255
};

enum SyndocalAsioEventSeverityV2 {
    SYNDOCAL_ASIO_V2_EVENT_WARNING = 1,
    SYNDOCAL_ASIO_V2_EVENT_TERMINAL = 2
};

enum SyndocalAsioEventKindV2 {
    SYNDOCAL_ASIO_V2_EVENT_XRUN = 1,
    SYNDOCAL_ASIO_V2_EVENT_RESET = 2,
    SYNDOCAL_ASIO_V2_EVENT_RESYNC = 3,
    SYNDOCAL_ASIO_V2_EVENT_SAMPLE_RATE_CHANGED = 4,
    SYNDOCAL_ASIO_V2_EVENT_DEVICE_LOST = 5,
    SYNDOCAL_ASIO_V2_EVENT_CALLBACK_GAP = 6,
    SYNDOCAL_ASIO_V2_EVENT_REALTIME_DENIED = 7,
    SYNDOCAL_ASIO_V2_EVENT_BACKEND = 8,
    SYNDOCAL_ASIO_V2_EVENT_MALFORMED_CALLBACK = 9,
    SYNDOCAL_ASIO_V2_EVENT_BUFFER_SIZE_CHANGED = 10
};

typedef void(SYNDOCAL_ASIO_CALL *SyndocalAsioSampleCallbackV2)(
    void *context,
    const float *mono_samples,
    size_t len,
    uint64_t capture_delay_ns,
    uint32_t callback_frames);

typedef void(SYNDOCAL_ASIO_CALL *SyndocalAsioEventCallbackV2)(
    void *context,
    uint32_t severity,
    uint32_t kind,
    const uint8_t *message,
    size_t message_len);

uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_abi_version(void);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_build_flags(void);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_drivers_json(
    SyndocalAsioStringV2 *out_json,
    SyndocalAsioStringV2 *out_error_json);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_capabilities_json(
    const uint8_t *request_json,
    size_t request_json_len,
    SyndocalAsioStringV2 *out_json,
    SyndocalAsioStringV2 *out_error_json);
void SYNDOCAL_ASIO_CALL syndocal_asio_v2_string_free(SyndocalAsioStringV2 string);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_start(
    const uint8_t *request_json,
    size_t request_json_len,
    SyndocalAsioSampleCallbackV2 sample_callback,
    SyndocalAsioEventCallbackV2 event_callback,
    void *context,
    SyndocalAsioHandle **out_handle,
    SyndocalAsioStringV2 *out_result_json,
    SyndocalAsioStringV2 *out_error_json);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_stop(
    SyndocalAsioHandle *handle,
    SyndocalAsioStringV2 *out_result_json,
    SyndocalAsioStringV2 *out_error_json);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_close(
    SyndocalAsioHandle **handle,
    SyndocalAsioStringV2 *out_result_json,
    SyndocalAsioStringV2 *out_error_json);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_telemetry_json(
    const SyndocalAsioHandle *handle,
    SyndocalAsioStringV2 *out_json,
    SyndocalAsioStringV2 *out_error_json);

#ifdef __cplusplus
}
#endif

#endif
