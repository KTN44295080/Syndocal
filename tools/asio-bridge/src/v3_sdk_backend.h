#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum SdAsioV3NativeFormat {
    SD_ASIO_V3_I16_LE = 1,
    SD_ASIO_V3_I16_BE = 2,
    SD_ASIO_V3_I24_LE = 3,
    SD_ASIO_V3_I24_BE = 4,
    SD_ASIO_V3_I32_LE = 5,
    SD_ASIO_V3_I32_BE = 6,
    SD_ASIO_V3_F32_LE = 7,
    SD_ASIO_V3_F32_BE = 8,
    SD_ASIO_V3_F64_LE = 9,
    SD_ASIO_V3_F64_BE = 10
};

typedef uint32_t (*SdAsioV3ProcessCallback)(
    void *context,
    const void *const *input_buffers,
    void *const *output_buffers,
    uint32_t input_format,
    uint32_t output_format,
    uint32_t frames,
    uint64_t first_output_frame,
    uint64_t session_generation,
    uint64_t render_generation);

typedef struct SdAsioV3StartConfig {
    const char *driver_name;
    uint32_t output_channels;
    uint32_t input_channels;
    uint32_t output_format;
    uint32_t input_format;
    uint32_t input_sample_rate_hz;
    uint32_t input_fixed_buffer_frames;
    uint32_t sample_rate_hz;
    uint32_t fixed_buffer_frames;
    uint64_t first_output_frame;
    uint64_t session_generation;
    uint64_t render_generation;
    SdAsioV3ProcessCallback process_callback;
    void *process_context;
} SdAsioV3StartConfig;

typedef struct SdAsioV3Telemetry {
    uint64_t callbacks;
    uint64_t xruns;
    uint64_t last_callback_tick_ms;
    uint32_t terminal_kind;
    uint32_t running;
} SdAsioV3Telemetry;

typedef struct SdAsioV3Started {
    uint32_t output_format;
    uint32_t input_format;
    uint32_t output_channels;
    uint32_t input_channels;
    uint32_t sample_rate_hz;
    uint32_t fixed_buffer_frames;
} SdAsioV3Started;

typedef struct SdAsioV3Capabilities {
    uint32_t input_channels;
    uint32_t output_channels;
    uint32_t input_format;
    uint32_t output_format;
    uint32_t current_sample_rate_hz;
    uint32_t buffer_min_frames;
    uint32_t buffer_max_frames;
    uint32_t buffer_preferred_frames;
    int32_t buffer_granularity;
    uint32_t supported_rate_count;
    uint32_t supported_rates_hz[24];
} SdAsioV3Capabilities;

int32_t sd_asio_v3_driver_names(
    char *slots,
    size_t slot_size,
    size_t max_drivers,
    size_t *out_count,
    char *error,
    size_t error_len);

int32_t sd_asio_v3_capabilities(
    const char *driver_name,
    SdAsioV3Capabilities *out_capabilities,
    char *error,
    size_t error_len);

int32_t sd_asio_v3_start(
    const SdAsioV3StartConfig *config,
    void **out_session,
    SdAsioV3Started *out_started,
    char *error,
    size_t error_len);

int32_t sd_asio_v3_stop(void *session, char *error, size_t error_len);
int32_t sd_asio_v3_close(void **session, char *error, size_t error_len);
void sd_asio_v3_telemetry(const void *session, SdAsioV3Telemetry *out_telemetry);
void sd_asio_v3_latch_terminal(void *session, uint32_t terminal_kind);
uint64_t sd_asio_v3_driver_open_attempts(void);
uint32_t sd_asio_v3_has_published_session(void);

#ifdef __cplusplus
}
#endif
