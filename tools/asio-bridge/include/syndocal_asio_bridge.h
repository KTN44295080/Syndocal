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
typedef struct SyndocalAsioV3Handle SyndocalAsioV3Handle;

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

/*
 * ABI v3 is a separate output/full-duplex boundary.  It intentionally does
 * not alter any ABI v2 declaration above.  PROGRAM/CUE physical channel
 * mapping is application-owned machine state, never part of this wire schema.
 */
#define SYNDOCAL_ASIO_V3_ABI_VERSION 3u
#define SYNDOCAL_ASIO_V3_JSON_SCHEMA_VERSION 3u

typedef struct SyndocalAsioStringV3 {
    uint8_t *ptr;
    size_t len;
} SyndocalAsioStringV3;

enum SyndocalAsioStatusV3 {
    SYNDOCAL_ASIO_V3_OK = 0,
    SYNDOCAL_ASIO_V3_INVALID_ARGUMENT = 1,
    SYNDOCAL_ASIO_V3_UNSUPPORTED = 2,
    SYNDOCAL_ASIO_V3_BACKEND_ERROR = 5,
    SYNDOCAL_ASIO_V3_TERMINAL = 6,
    SYNDOCAL_ASIO_V3_PANIC = 255
};

/*
 * Exact callback result values.  The callback receives normalized f32 frames;
 * the bridge alone owns conversion to/from the requested native ASIO format.
 * ACCEPTED means the callback filled the full device-width block for the exact
 * supplied frame/generation metadata.  Every other result requires the bridge
 * to write a complete device-width silent block and latch a terminal fault;
 * no partial block, retry, fallback device, or callback-time allocation is
 * permitted.
 * A fullDuplex Start uses one ASIO stream and clock: input.sampleRateHz and
 * input.fixedBufferFrames must exactly equal the corresponding output values.
 * A mismatch is rejected before driver open and never produces actualInput.
 */
enum SyndocalAsioCallbackResultV3 {
    SYNDOCAL_ASIO_V3_CALLBACK_ACCEPTED = 0,
    SYNDOCAL_ASIO_V3_CALLBACK_QUEUE_UNDERFLOW = 1,
    SYNDOCAL_ASIO_V3_CALLBACK_QUEUE_FULL = 2,
    SYNDOCAL_ASIO_V3_CALLBACK_TERMINAL = 3,
    SYNDOCAL_ASIO_V3_CALLBACK_INVALID_BLOCK = 4,
    SYNDOCAL_ASIO_V3_CALLBACK_PANIC = 5
};

enum SyndocalAsioEventSeverityV3 {
    SYNDOCAL_ASIO_V3_EVENT_WARNING = 1,
    SYNDOCAL_ASIO_V3_EVENT_TERMINAL = 2
};

/* Terminal conditions are never recoverable by automatic reconnect/restart. */
enum SyndocalAsioEventKindV3 {
    SYNDOCAL_ASIO_V3_EVENT_XRUN = 1,
    SYNDOCAL_ASIO_V3_EVENT_RESET = 2,
    SYNDOCAL_ASIO_V3_EVENT_RESYNC = 3,
    SYNDOCAL_ASIO_V3_EVENT_SAMPLE_RATE_CHANGED = 4,
    SYNDOCAL_ASIO_V3_EVENT_DEVICE_LOST = 5,
    SYNDOCAL_ASIO_V3_EVENT_CALLBACK_GAP = 6,
    SYNDOCAL_ASIO_V3_EVENT_REALTIME_DENIED = 7,
    SYNDOCAL_ASIO_V3_EVENT_BACKEND = 8,
    SYNDOCAL_ASIO_V3_EVENT_MALFORMED_CALLBACK = 9,
    SYNDOCAL_ASIO_V3_EVENT_BUFFER_SIZE_CHANGED = 10,
    SYNDOCAL_ASIO_V3_EVENT_OUTPUT_QUEUE_UNDERFLOW = 11,
    SYNDOCAL_ASIO_V3_EVENT_OUTPUT_QUEUE_FULL = 12,
    SYNDOCAL_ASIO_V3_EVENT_CALLBACK_PANIC = 13,
    SYNDOCAL_ASIO_V3_EVENT_INVALID_OUTPUT_BLOCK = 14
};

/*
 * `context` remains owned by the caller from Start until successful Stop or
 * Close.  On successful Stop/Close the bridge first unpublishes dispatch and
 * drains in-flight callback readers; neither output nor event callback can
 * begin or continue after that successful return. `session_generation` and
 * `render_generation` are immutable ASIO-session root fences copied from the
 * successful Start request. Transport/bus generation changes do not restart
 * the device and must be fenced lock-free by caller-owned atomics and queued
 * block metadata reachable through `context`.
 */
typedef uint32_t(SYNDOCAL_ASIO_CALL *SyndocalAsioOutputCallbackV3)(
    void *context,
    float *interleaved_output,
    uint32_t output_channels,
    uint32_t frames,
    uint64_t first_output_frame,
    uint64_t session_generation,
    uint64_t render_generation);

typedef uint32_t(SYNDOCAL_ASIO_CALL *SyndocalAsioDuplexCallbackV3)(
    void *context,
    const float *interleaved_input,
    uint32_t input_channels,
    float *interleaved_output,
    uint32_t output_channels,
    uint32_t frames,
    uint64_t first_output_frame,
    uint64_t session_generation,
    uint64_t render_generation);

typedef void(SYNDOCAL_ASIO_CALL *SyndocalAsioEventCallbackV3)(
    void *context,
    uint32_t severity,
    uint32_t kind,
    const uint8_t *message,
    size_t message_len);

/*
 * For every v3 JSON operation, each non-null out string must initially be
 * `{NULL, 0}` and no two output slots may overlap in memory, including an
 * interior/partial overlap.  `out_handle`, `out_result_json`, and
 * `out_error_json` of Start/Close must likewise be pairwise non-overlapping.
 * On alias rejection the bridge writes no output slot.  A returned bridge
 * string is owned by the caller and must be passed exactly once to
 * `syndocal_asio_v3_string_free`; it must never be freed by another allocator
 * or retained after that call.
 */
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_abi_version(void);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_build_flags(void);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_drivers_json(
    SyndocalAsioStringV3 *out_json,
    SyndocalAsioStringV3 *out_error_json);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_capabilities_json(
    const uint8_t *request_json,
    size_t request_json_len,
    SyndocalAsioStringV3 *out_json,
    SyndocalAsioStringV3 *out_error_json);
void SYNDOCAL_ASIO_CALL syndocal_asio_v3_string_free(SyndocalAsioStringV3 string);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_start(
    const uint8_t *request_json,
    size_t request_json_len,
    SyndocalAsioOutputCallbackV3 output_callback,
    SyndocalAsioDuplexCallbackV3 duplex_callback,
    SyndocalAsioEventCallbackV3 event_callback,
    void *context,
    SyndocalAsioV3Handle **out_handle,
    SyndocalAsioStringV3 *out_result_json,
    SyndocalAsioStringV3 *out_error_json);
/* A successful Stop/Close guarantees callback quiescence before it returns. */
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_stop(
    SyndocalAsioV3Handle *handle,
    SyndocalAsioStringV3 *out_result_json,
    SyndocalAsioStringV3 *out_error_json);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_close(
    SyndocalAsioV3Handle **handle,
    SyndocalAsioStringV3 *out_result_json,
    SyndocalAsioStringV3 *out_error_json);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_telemetry_json(
    const SyndocalAsioV3Handle *handle,
    SyndocalAsioStringV3 *out_json,
    SyndocalAsioStringV3 *out_error_json);

#ifdef __cplusplus
}
#endif

#endif
