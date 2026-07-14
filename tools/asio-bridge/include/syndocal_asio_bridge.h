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

#define SYNDOCAL_ASIO_ABI_VERSION 1u
#define SYNDOCAL_ASIO_BUILD_FLAG_ASIO_COMPILED 0x1u

typedef struct SyndocalAsioHandle SyndocalAsioHandle;

typedef struct SyndocalAsioStringV1 {
    uint8_t *ptr;
    size_t len;
} SyndocalAsioStringV1;

typedef struct SyndocalAsioChannelMixV1 {
    uint32_t channel_index;
    float gain;
} SyndocalAsioChannelMixV1;

typedef struct SyndocalAsioStartConfigV1 {
    uint32_t struct_size;
    uint32_t abi_version;
    const uint8_t *driver_id;
    size_t driver_id_len;
    uint32_t sample_rate_hz;
    uint32_t input_channels;
    uint32_t sample_format;
    uint32_t fixed_buffer_frames;
    const SyndocalAsioChannelMixV1 *channel_mix;
    size_t channel_mix_len;
    uint32_t flags;
    uint32_t reserved;
} SyndocalAsioStartConfigV1;

enum SyndocalAsioStatusV1 {
    SYNDOCAL_ASIO_OK = 0,
    SYNDOCAL_ASIO_INVALID_ARGUMENT = 1,
    SYNDOCAL_ASIO_UNSUPPORTED = 2,
    SYNDOCAL_ASIO_DRIVER_NOT_FOUND = 3,
    SYNDOCAL_ASIO_CONFIG_UNSUPPORTED = 4,
    SYNDOCAL_ASIO_BACKEND_ERROR = 5,
    SYNDOCAL_ASIO_TERMINAL = 6,
    SYNDOCAL_ASIO_PANIC = 255
};

enum SyndocalAsioSampleFormatV1 {
    SYNDOCAL_ASIO_SAMPLE_F32 = 1,
    SYNDOCAL_ASIO_SAMPLE_I16 = 2,
    SYNDOCAL_ASIO_SAMPLE_I24 = 3,
    SYNDOCAL_ASIO_SAMPLE_I32 = 4,
    SYNDOCAL_ASIO_SAMPLE_F64 = 5
};

enum SyndocalAsioEventSeverityV1 {
    SYNDOCAL_ASIO_EVENT_WARNING = 1,
    SYNDOCAL_ASIO_EVENT_TERMINAL = 2
};

enum SyndocalAsioEventKindV1 {
    SYNDOCAL_ASIO_EVENT_XRUN = 1,
    SYNDOCAL_ASIO_EVENT_RESET = 2,
    SYNDOCAL_ASIO_EVENT_RESYNC = 3,
    SYNDOCAL_ASIO_EVENT_SAMPLE_RATE_CHANGED = 4,
    SYNDOCAL_ASIO_EVENT_DEVICE_LOST = 5,
    SYNDOCAL_ASIO_EVENT_CALLBACK_GAP = 6,
    SYNDOCAL_ASIO_EVENT_REALTIME_DENIED = 7,
    SYNDOCAL_ASIO_EVENT_BACKEND = 8,
    SYNDOCAL_ASIO_EVENT_MALFORMED_CALLBACK = 9,
    SYNDOCAL_ASIO_EVENT_BUFFER_SIZE_CHANGED = 10
};

typedef void(SYNDOCAL_ASIO_CALL *SyndocalAsioSampleCallbackV1)(
    void *context,
    const float *mono_samples,
    size_t len,
    uint64_t capture_delay_ns,
    uint32_t callback_frames);

typedef void(SYNDOCAL_ASIO_CALL *SyndocalAsioEventCallbackV1)(
    void *context,
    uint32_t severity,
    uint32_t kind,
    const uint8_t *message,
    size_t message_len);

uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_abi_version(void);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_build_flags(void);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_drivers_json(
    SyndocalAsioStringV1 *out_json,
    SyndocalAsioStringV1 *out_error_json);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_capabilities_json(
    const uint8_t *driver_id,
    size_t driver_id_len,
    SyndocalAsioStringV1 *out_json,
    SyndocalAsioStringV1 *out_error_json);
void SYNDOCAL_ASIO_CALL syndocal_asio_string_free(SyndocalAsioStringV1 string);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_start(
    const SyndocalAsioStartConfigV1 *config,
    SyndocalAsioSampleCallbackV1 sample_callback,
    SyndocalAsioEventCallbackV1 event_callback,
    void *context,
    SyndocalAsioHandle **out_handle,
    SyndocalAsioStringV1 *out_error_json);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_actual_buffer_frames(
    const SyndocalAsioHandle *handle,
    uint32_t *out_frames);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_play(
    SyndocalAsioHandle *handle,
    SyndocalAsioStringV1 *out_error_json);
uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_stop(
    SyndocalAsioHandle *handle,
    SyndocalAsioStringV1 *out_error_json);
uint64_t SYNDOCAL_ASIO_CALL syndocal_asio_xrun_count(const SyndocalAsioHandle *handle);
void SYNDOCAL_ASIO_CALL syndocal_asio_free(SyndocalAsioHandle *handle);

#ifdef __cplusplus
}
#endif

#endif
