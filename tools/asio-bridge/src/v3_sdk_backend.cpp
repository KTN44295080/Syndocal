#include "v3_sdk_backend.h"

#include "asio.h"
#include "asiodrivers.h"
#include "asiosys.h"

#include <windows.h>

#include <atomic>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <limits>
#include <new>
#include <vector>

extern "C" bool load_asio_driver(char *name);
extern "C" void remove_current_driver();
extern "C" long get_driver_names(char **names, long maxDrivers);
extern "C" ASIOError get_sample_rate(double *rate);
extern "C" ASIOError set_sample_rate(double rate);
extern "C" ASIOError can_sample_rate(double rate);

namespace {

constexpr uint32_t kEventXrun = 1;
constexpr uint32_t kEventReset = 2;
constexpr uint32_t kEventResync = 3;
constexpr uint32_t kEventSampleRateChanged = 4;
constexpr uint32_t kEventDeviceLost = 5;
constexpr uint32_t kEventCallbackGap = 6;
constexpr uint32_t kEventBackend = 8;
constexpr uint32_t kEventMalformedCallback = 9;
constexpr uint32_t kEventBufferSizeChanged = 10;
constexpr uint32_t kEventQueueUnderflow = 11;
constexpr uint32_t kEventQueueFull = 12;
constexpr uint32_t kEventCallbackPanic = 13;
constexpr uint32_t kEventInvalidOutput = 14;

constexpr uint32_t kCallbackAccepted = 0;
constexpr uint32_t kCallbackQueueUnderflow = 1;
constexpr uint32_t kCallbackQueueFull = 2;
constexpr uint32_t kCallbackTerminal = 3;
constexpr uint32_t kCallbackInvalidBlock = 4;
constexpr uint32_t kCallbackPanic = 5;

struct Session {
    std::vector<ASIOBufferInfo> buffer_infos;
    std::vector<void *> input_buffers[2];
    std::vector<void *> output_buffers[2];
    uint32_t input_channels = 0;
    uint32_t output_channels = 0;
    uint32_t input_format = 0;
    uint32_t output_format = 0;
    uint32_t sample_rate_hz = 0;
    uint32_t frames = 0;
    uint64_t session_generation = 0;
    uint64_t render_generation = 0;
    SdAsioV3ProcessCallback process_callback = nullptr;
    void *process_context = nullptr;
    bool output_ready = false;
    bool driver_loaded = false;
    bool initialized = false;
    bool buffers_created = false;
    std::atomic<bool> running{false};
    bool closed = false;
    std::atomic<bool> accept_client{false};
    std::atomic<uint64_t> next_output_frame{0};
    std::atomic<uint64_t> callbacks{0};
    std::atomic<uint64_t> xruns{0};
    std::atomic<uint64_t> last_callback_tick_ms{0};
    std::atomic<uint32_t> terminal_kind{0};
};

std::atomic<Session *> g_published{nullptr};
std::atomic<uint32_t> g_readers{0};
std::atomic<uint64_t> g_driver_open_attempts{0};

void write_error(char *error, size_t error_len, const char *message) {
    if (error == nullptr || error_len == 0) {
        return;
    }
    std::snprintf(error, error_len, "%s", message == nullptr ? "ASIO backend error" : message);
    error[error_len - 1] = '\0';
}

void write_asio_error(char *error, size_t error_len, const char *operation, ASIOError code) {
    if (error == nullptr || error_len == 0) {
        return;
    }
    std::snprintf(error, error_len, "%s failed with ASIOError %ld", operation, static_cast<long>(code));
    error[error_len - 1] = '\0';
}

uint32_t sample_size(uint32_t format) {
    switch (format) {
    case SD_ASIO_V3_I16_LE:
    case SD_ASIO_V3_I16_BE:
        return 2;
    case SD_ASIO_V3_I24_LE:
    case SD_ASIO_V3_I24_BE:
        return 3;
    case SD_ASIO_V3_I32_LE:
    case SD_ASIO_V3_I32_BE:
    case SD_ASIO_V3_F32_LE:
    case SD_ASIO_V3_F32_BE:
        return 4;
    case SD_ASIO_V3_F64_LE:
    case SD_ASIO_V3_F64_BE:
        return 8;
    default:
        return 0;
    }
}

uint32_t format_family(uint32_t format) {
    switch (format) {
    case SD_ASIO_V3_I16_LE:
    case SD_ASIO_V3_I16_BE:
        return 1;
    case SD_ASIO_V3_I24_LE:
    case SD_ASIO_V3_I24_BE:
        return 2;
    case SD_ASIO_V3_I32_LE:
    case SD_ASIO_V3_I32_BE:
        return 3;
    case SD_ASIO_V3_F32_LE:
    case SD_ASIO_V3_F32_BE:
        return 4;
    case SD_ASIO_V3_F64_LE:
    case SD_ASIO_V3_F64_BE:
        return 5;
    default:
        return 0;
    }
}

uint32_t native_format(ASIOSampleType type) {
    switch (type) {
    case ASIOSTInt16LSB:
        return SD_ASIO_V3_I16_LE;
    case ASIOSTInt16MSB:
        return SD_ASIO_V3_I16_BE;
    case ASIOSTInt24LSB:
        return SD_ASIO_V3_I24_LE;
    case ASIOSTInt24MSB:
        return SD_ASIO_V3_I24_BE;
    case ASIOSTInt32LSB:
        return SD_ASIO_V3_I32_LE;
    case ASIOSTInt32MSB:
        return SD_ASIO_V3_I32_BE;
    case ASIOSTFloat32LSB:
        return SD_ASIO_V3_F32_LE;
    case ASIOSTFloat32MSB:
        return SD_ASIO_V3_F32_BE;
    case ASIOSTFloat64LSB:
        return SD_ASIO_V3_F64_LE;
    case ASIOSTFloat64MSB:
        return SD_ASIO_V3_F64_BE;
    default:
        return 0;
    }
}

void latch_terminal(Session *session, uint32_t kind) {
    if (session == nullptr || kind == 0) {
        return;
    }
    uint32_t expected = 0;
    if (session->terminal_kind.compare_exchange_strong(expected, kind, std::memory_order_acq_rel)) {
        if (kind == kEventXrun) {
            session->xruns.fetch_add(1, std::memory_order_relaxed);
        }
    }
}

void latch_published_terminal(uint32_t kind) {
    g_readers.fetch_add(1, std::memory_order_acq_rel);
    Session *session = g_published.load(std::memory_order_acquire);
    latch_terminal(session, kind);
    g_readers.fetch_sub(1, std::memory_order_release);
}

void silence_output(Session *session, uint32_t buffer_index) {
    if (session == nullptr || buffer_index > 1) {
        return;
    }
    const uint32_t bytes = sample_size(session->output_format);
    if (bytes == 0) {
        latch_terminal(session, kEventMalformedCallback);
        return;
    }
    const size_t channel_bytes = static_cast<size_t>(session->frames) * bytes;
    for (void *buffer : session->output_buffers[buffer_index]) {
        if (buffer == nullptr) {
            latch_terminal(session, kEventMalformedCallback);
            continue;
        }
        std::memset(buffer, 0, channel_bytes);
    }
}

uint32_t callback_kind(uint32_t result) {
    switch (result) {
    case kCallbackQueueUnderflow:
        return kEventQueueUnderflow;
    case kCallbackQueueFull:
        return kEventQueueFull;
    case kCallbackTerminal:
        return kEventBackend;
    case kCallbackInvalidBlock:
        return kEventInvalidOutput;
    case kCallbackPanic:
        return kEventCallbackPanic;
    default:
        return kEventMalformedCallback;
    }
}

void process_buffer(long double_buffer_index) noexcept {
    g_readers.fetch_add(1, std::memory_order_acq_rel);
    Session *session = g_published.load(std::memory_order_acquire);
    if (session == nullptr) {
        g_readers.fetch_sub(1, std::memory_order_release);
        return;
    }

    const uint32_t index = static_cast<uint32_t>(double_buffer_index);
    if (index > 1) {
        latch_terminal(session, kEventMalformedCallback);
        g_readers.fetch_sub(1, std::memory_order_release);
        return;
    }

    session->last_callback_tick_ms.store(GetTickCount64(), std::memory_order_release);
    const uint64_t first_frame =
        session->next_output_frame.fetch_add(session->frames, std::memory_order_acq_rel);
    uint32_t result = kCallbackTerminal;
    try {
        if (session->accept_client.load(std::memory_order_acquire) &&
            session->terminal_kind.load(std::memory_order_acquire) == 0 &&
            session->process_callback != nullptr) {
            result = session->process_callback(
                session->process_context,
                session->input_channels == 0 ? nullptr : session->input_buffers[index].data(),
                session->output_buffers[index].data(),
                session->input_format,
                session->output_format,
                session->frames,
                first_frame,
                session->session_generation,
                session->render_generation);
        }
    } catch (...) {
        result = kCallbackPanic;
    }

    if (result != kCallbackAccepted) {
        silence_output(session, index);
        latch_terminal(session, callback_kind(result));
    }
    session->callbacks.fetch_add(1, std::memory_order_release);
    if (session->output_ready) {
        ASIOOutputReady();
    }
    g_readers.fetch_sub(1, std::memory_order_release);
}

void buffer_switch(long double_buffer_index, ASIOBool) {
    process_buffer(double_buffer_index);
}

ASIOTime *buffer_switch_time_info(ASIOTime *time_info, long double_buffer_index, ASIOBool) {
    process_buffer(double_buffer_index);
    return time_info;
}

void sample_rate_changed(ASIOSampleRate) {
    latch_published_terminal(kEventSampleRateChanged);
}

long asio_message(long selector, long value, void *, double *) {
    switch (selector) {
    case kAsioSelectorSupported:
        switch (value) {
        case kAsioResetRequest:
        case kAsioBufferSizeChange:
        case kAsioResyncRequest:
        case kAsioLatenciesChanged:
        case kAsioEngineVersion:
        case kAsioSupportsTimeInfo:
        case kAsioOverload:
            return 1;
        default:
            return 0;
        }
    case kAsioResetRequest:
        latch_published_terminal(kEventReset);
        return 1;
    case kAsioBufferSizeChange:
    case kAsioLatenciesChanged:
        latch_published_terminal(kEventBufferSizeChanged);
        return 1;
    case kAsioResyncRequest:
        latch_published_terminal(kEventResync);
        return 1;
    case kAsioOverload:
        latch_published_terminal(kEventXrun);
        return 1;
    case kAsioEngineVersion:
        return 2;
    case kAsioSupportsTimeInfo:
        return 1;
    case kAsioSupportsTimeCode:
        return 0;
    default:
        return 0;
    }
}

ASIOCallbacks kCallbacks = {
    buffer_switch,
    sample_rate_changed,
    asio_message,
    buffer_switch_time_info,
};

bool buffer_size_supported(uint32_t frames, long minimum, long maximum, long granularity) {
    if (minimum <= 0 || maximum < minimum || frames < static_cast<uint32_t>(minimum) ||
        frames > static_cast<uint32_t>(maximum)) {
        return false;
    }
    if (granularity == -1) {
        return frames != 0 && (frames & (frames - 1)) == 0;
    }
    if (granularity > 0) {
        return (static_cast<long>(frames) - minimum) % granularity == 0;
    }
    return granularity == 0;
}

bool query_uniform_format(bool input, uint32_t channels, uint32_t *out_format,
                          char *error, size_t error_len) {
    if (channels == 0 || out_format == nullptr) {
        write_error(error, error_len, "ASIO channel count must not be zero");
        return false;
    }
    uint32_t uniform = 0;
    for (uint32_t channel = 0; channel < channels; ++channel) {
        ASIOChannelInfo info{};
        info.channel = static_cast<long>(channel);
        info.isInput = input ? ASIOTrue : ASIOFalse;
        ASIOError result = ASIOGetChannelInfo(&info);
        if (result != ASE_OK) {
            write_asio_error(error, error_len, "ASIOGetChannelInfo", result);
            return false;
        }
        const uint32_t format = native_format(info.type);
        if (format == 0) {
            write_error(error, error_len, "ASIO channel uses an unsupported native sample format");
            return false;
        }
        if (uniform == 0) {
            uniform = format;
        } else if (uniform != format) {
            write_error(error, error_len, "ASIO device channels do not share one exact native format");
            return false;
        }
    }
    *out_format = uniform;
    return true;
}

bool init_driver(const char *name, bool *loaded, bool *initialized,
                 char *error, size_t error_len) {
    if (name == nullptr || name[0] == '\0') {
        write_error(error, error_len, "explicit ASIO driver name is empty");
        return false;
    }
    if (!load_asio_driver(const_cast<char *>(name))) {
        write_error(error, error_len, "explicit ASIO driver could not be loaded");
        return false;
    }
    *loaded = true;
    ASIODriverInfo driver_info{};
    driver_info.asioVersion = 2;
    driver_info.sysRef = GetDesktopWindow();
    const ASIOError result = ASIOInit(&driver_info);
    if (result != ASE_OK) {
        write_asio_error(error, error_len, "ASIOInit", result);
        return false;
    }
    *initialized = true;
    return true;
}

bool exit_driver(bool *loaded, bool *initialized, char *error, size_t error_len) {
    if (*initialized) {
        const ASIOError result = ASIOExit();
        if (result != ASE_OK) {
            write_asio_error(error, error_len, "ASIOExit", result);
            return false;
        }
        *initialized = false;
    }
    if (*loaded) {
        remove_current_driver();
        *loaded = false;
    }
    return true;
}

bool configure_exact_rate(uint32_t rate, char *error, size_t error_len) {
    if (rate == 0 || can_sample_rate(static_cast<double>(rate)) != ASE_OK) {
        write_error(error, error_len, "ASIO driver does not support the exact requested sample rate");
        return false;
    }
    double actual = 0.0;
    ASIOError result = get_sample_rate(&actual);
    if (result != ASE_OK || std::fabs(actual - static_cast<double>(rate)) >= 0.5) {
        result = set_sample_rate(static_cast<double>(rate));
        if (result != ASE_OK) {
            write_asio_error(error, error_len, "ASIOSetSampleRate", result);
            return false;
        }
        result = get_sample_rate(&actual);
    }
    if (result != ASE_OK || std::fabs(actual - static_cast<double>(rate)) >= 0.5) {
        write_error(error, error_len, "ASIO driver did not apply the exact requested sample rate");
        return false;
    }
    return true;
}

bool cleanup_session(Session *session, char *error, size_t error_len) {
    if (session == nullptr) {
        return true;
    }
    session->accept_client.store(false, std::memory_order_release);
    if (session->running.load(std::memory_order_acquire)) {
        const ASIOError result = ASIOStop();
        if (result != ASE_OK) {
            write_asio_error(error, error_len, "ASIOStop", result);
            latch_terminal(session, kEventDeviceLost);
            return false;
        }
        session->running.store(false, std::memory_order_release);
    }

    Session *expected = session;
    g_published.compare_exchange_strong(expected, nullptr, std::memory_order_acq_rel);
    while (g_readers.load(std::memory_order_acquire) != 0) {
        SwitchToThread();
    }

    if (session->buffers_created) {
        const ASIOError result = ASIODisposeBuffers();
        if (result != ASE_OK) {
            write_asio_error(error, error_len, "ASIODisposeBuffers", result);
            latch_terminal(session, kEventBackend);
            return false;
        }
        session->buffers_created = false;
    }
    if (!exit_driver(&session->driver_loaded, &session->initialized, error, error_len)) {
        latch_terminal(session, kEventBackend);
        return false;
    }
    session->closed = true;
    return true;
}

} // namespace

extern "C" int32_t sd_asio_v3_driver_names(
    char *slots, size_t slot_size, size_t max_drivers, size_t *out_count,
    char *error, size_t error_len) {
    if (slots == nullptr || out_count == nullptr || slot_size < 33 || max_drivers == 0 ||
        max_drivers > static_cast<size_t>(std::numeric_limits<long>::max())) {
        write_error(error, error_len, "invalid ASIO driver catalog output buffer");
        return 1;
    }
    std::memset(slots, 0, slot_size * max_drivers);
    std::vector<char *> pointers(max_drivers);
    for (size_t index = 0; index < max_drivers; ++index) {
        pointers[index] = slots + index * slot_size;
    }
    const long count = get_driver_names(pointers.data(), static_cast<long>(max_drivers));
    if (count < 0) {
        write_error(error, error_len, "ASIO driver enumeration failed");
        return 2;
    }
    *out_count = static_cast<size_t>(count);
    return 0;
}

extern "C" int32_t sd_asio_v3_capabilities(
    const char *driver_name, SdAsioV3Capabilities *out_capabilities,
    char *error, size_t error_len) {
    if (out_capabilities == nullptr) {
        write_error(error, error_len, "capabilities output is null");
        return 1;
    }
    std::memset(out_capabilities, 0, sizeof(*out_capabilities));
    bool loaded = false;
    bool initialized = false;
    if (!init_driver(driver_name, &loaded, &initialized, error, error_len)) {
        exit_driver(&loaded, &initialized, nullptr, 0);
        return 2;
    }

    int32_t status = 0;
    long inputs = 0;
    long outputs = 0;
    long minimum = 0;
    long maximum = 0;
    long preferred = 0;
    long granularity = 0;
    double current_rate = 0.0;
    if (ASIOGetChannels(&inputs, &outputs) != ASE_OK || inputs < 0 || outputs <= 0 ||
        ASIOGetBufferSize(&minimum, &maximum, &preferred, &granularity) != ASE_OK ||
        minimum <= 0 || maximum < minimum || preferred < minimum || preferred > maximum ||
        get_sample_rate(&current_rate) != ASE_OK || current_rate <= 0.0 ||
        current_rate > static_cast<double>(std::numeric_limits<uint32_t>::max())) {
        write_error(error, error_len, "ASIO driver returned invalid channel/rate/buffer capabilities");
        status = 3;
    } else {
        uint32_t input_format = 0;
        uint32_t output_format = 0;
        if (!query_uniform_format(false, static_cast<uint32_t>(outputs), &output_format, error, error_len) ||
            (inputs > 0 && !query_uniform_format(true, static_cast<uint32_t>(inputs), &input_format, error, error_len))) {
            status = 4;
        } else {
            out_capabilities->input_channels = static_cast<uint32_t>(inputs);
            out_capabilities->output_channels = static_cast<uint32_t>(outputs);
            out_capabilities->input_format = input_format;
            out_capabilities->output_format = output_format;
            out_capabilities->current_sample_rate_hz = static_cast<uint32_t>(std::llround(current_rate));
            out_capabilities->buffer_min_frames = static_cast<uint32_t>(minimum);
            out_capabilities->buffer_max_frames = static_cast<uint32_t>(maximum);
            out_capabilities->buffer_preferred_frames = static_cast<uint32_t>(preferred);
            out_capabilities->buffer_granularity = static_cast<int32_t>(granularity);
            const uint32_t candidates[] = {
                8000, 11025, 16000, 22050, 24000, 32000, 44100, 48000,
                64000, 88200, 96000, 128000, 176400, 192000, 352800, 384000,
                705600, 768000,
            };
            for (uint32_t rate : candidates) {
                if (can_sample_rate(static_cast<double>(rate)) == ASE_OK &&
                    out_capabilities->supported_rate_count < 24) {
                    out_capabilities->supported_rates_hz[out_capabilities->supported_rate_count++] = rate;
                }
            }
            const uint32_t rounded_current = static_cast<uint32_t>(std::llround(current_rate));
            bool current_recorded = false;
            for (uint32_t index = 0; index < out_capabilities->supported_rate_count; ++index) {
                current_recorded |= out_capabilities->supported_rates_hz[index] == rounded_current;
            }
            if (!current_recorded && std::fabs(current_rate - static_cast<double>(rounded_current)) < 0.5 &&
                can_sample_rate(static_cast<double>(rounded_current)) == ASE_OK &&
                out_capabilities->supported_rate_count < 24) {
                out_capabilities->supported_rates_hz[out_capabilities->supported_rate_count++] = rounded_current;
            }
        }
    }
    if (!exit_driver(&loaded, &initialized, error, error_len) && status == 0) {
        status = 5;
    }
    return status;
}

extern "C" int32_t sd_asio_v3_start(
    const SdAsioV3StartConfig *config, void **out_session, SdAsioV3Started *out_started,
    char *error, size_t error_len) {
    if (config == nullptr || out_session == nullptr || out_started == nullptr ||
        *out_session != nullptr || config->process_callback == nullptr ||
        config->output_channels == 0 || config->fixed_buffer_frames == 0 ||
        config->sample_rate_hz == 0 || format_family(config->output_format) == 0 ||
        (config->input_channels > 0 && format_family(config->input_format) == 0)) {
        write_error(error, error_len, "invalid ASIO v3 SDK start contract");
        return 1;
    }
    if ((config->input_channels == 0 &&
         (config->input_format != 0 || config->input_sample_rate_hz != 0 ||
          config->input_fixed_buffer_frames != 0)) ||
        (config->input_channels > 0 &&
         (config->input_sample_rate_hz == 0 || config->input_fixed_buffer_frames == 0))) {
        write_error(error, error_len, "invalid ASIO v3 input tuple presence contract");
        return 1;
    }
    if (config->input_channels > 0 && config->input_sample_rate_hz != config->sample_rate_hz) {
        write_error(error, error_len,
                    "full-duplex input and output sample rates must match the shared ASIO clock");
        return 1;
    }
    if (config->input_channels > 0 &&
        config->input_fixed_buffer_frames != config->fixed_buffer_frames) {
        write_error(error, error_len,
                    "full-duplex input and output buffer sizes must match the shared ASIO callback");
        return 1;
    }
    if (g_published.load(std::memory_order_acquire) != nullptr) {
        write_error(error, error_len, "an ASIO v3 SDK session is already published");
        return 2;
    }

    Session *session = new (std::nothrow) Session();
    if (session == nullptr) {
        write_error(error, error_len, "ASIO v3 session allocation failed");
        return 3;
    }
    session->input_channels = config->input_channels;
    session->output_channels = config->output_channels;
    session->sample_rate_hz = config->sample_rate_hz;
    session->frames = config->fixed_buffer_frames;
    session->session_generation = config->session_generation;
    session->render_generation = config->render_generation;
    session->process_callback = config->process_callback;
    session->process_context = config->process_context;
    session->next_output_frame.store(config->first_output_frame, std::memory_order_relaxed);

    int32_t status = 0;
    g_driver_open_attempts.fetch_add(1, std::memory_order_acq_rel);
    if (!init_driver(config->driver_name, &session->driver_loaded, &session->initialized, error, error_len)) {
        status = 4;
    }

    long actual_inputs = 0;
    long actual_outputs = 0;
    long minimum = 0;
    long maximum = 0;
    long preferred = 0;
    long granularity = 0;
    if (status == 0 &&
        (ASIOGetChannels(&actual_inputs, &actual_outputs) != ASE_OK ||
         actual_inputs < 0 || actual_outputs <= 0 ||
         static_cast<uint32_t>(actual_outputs) != config->output_channels ||
         (config->input_channels > 0 && static_cast<uint32_t>(actual_inputs) != config->input_channels))) {
        write_error(error, error_len, "ASIO device channel count differs from the exact requested tuple");
        status = 5;
    }
    if (status == 0 && ASIOGetBufferSize(&minimum, &maximum, &preferred, &granularity) != ASE_OK) {
        write_error(error, error_len, "ASIOGetBufferSize failed during exact Start revalidation");
        status = 6;
    }
    if (status == 0 &&
        !buffer_size_supported(config->fixed_buffer_frames, minimum, maximum, granularity)) {
        write_error(error, error_len, "ASIO fixed buffer does not match the exact driver constraints");
        status = 7;
    }
    if (status == 0 && !configure_exact_rate(config->sample_rate_hz, error, error_len)) {
        status = 8;
    }
    if (status == 0 &&
        !query_uniform_format(false, config->output_channels, &session->output_format, error, error_len)) {
        status = 9;
    }
    if (status == 0 && format_family(session->output_format) != format_family(config->output_format)) {
        write_error(error, error_len, "ASIO output native format differs from the exact requested tuple");
        status = 10;
    }
    if (status == 0 && config->input_channels > 0 &&
        !query_uniform_format(true, config->input_channels, &session->input_format, error, error_len)) {
        status = 11;
    }
    if (status == 0 && config->input_channels > 0 &&
        format_family(session->input_format) != format_family(config->input_format)) {
        write_error(error, error_len, "ASIO input native format differs from the exact requested tuple");
        status = 12;
    }

    if (status == 0) {
        const size_t total_channels = static_cast<size_t>(config->input_channels) + config->output_channels;
        try {
            session->buffer_infos.resize(total_channels);
            for (uint32_t index = 0; index < config->input_channels; ++index) {
                ASIOBufferInfo &info = session->buffer_infos[index];
                info.isInput = ASIOTrue;
                info.channelNum = static_cast<long>(index);
                info.buffers[0] = nullptr;
                info.buffers[1] = nullptr;
            }
            for (uint32_t index = 0; index < config->output_channels; ++index) {
                ASIOBufferInfo &info = session->buffer_infos[config->input_channels + index];
                info.isInput = ASIOFalse;
                info.channelNum = static_cast<long>(index);
                info.buffers[0] = nullptr;
                info.buffers[1] = nullptr;
            }
        } catch (...) {
            write_error(error, error_len, "ASIO buffer metadata allocation failed before Start");
            status = 13;
        }
    }

    if (status == 0) {
        const ASIOError result = ASIOCreateBuffers(
            session->buffer_infos.data(), static_cast<long>(session->buffer_infos.size()),
            static_cast<long>(config->fixed_buffer_frames), &kCallbacks);
        if (result != ASE_OK) {
            write_asio_error(error, error_len, "ASIOCreateBuffers", result);
            status = 14;
        } else {
            session->buffers_created = true;
        }
    }

    if (status == 0) {
        try {
            for (uint32_t half = 0; half < 2; ++half) {
                session->input_buffers[half].reserve(config->input_channels);
                session->output_buffers[half].reserve(config->output_channels);
                for (uint32_t index = 0; index < config->input_channels; ++index) {
                    session->input_buffers[half].push_back(session->buffer_infos[index].buffers[half]);
                }
                for (uint32_t index = 0; index < config->output_channels; ++index) {
                    session->output_buffers[half].push_back(
                        session->buffer_infos[config->input_channels + index].buffers[half]);
                }
            }
        } catch (...) {
            write_error(error, error_len, "ASIO callback pointer table allocation failed before Start");
            status = 15;
        }
    }

    if (status == 0) {
        silence_output(session, 0);
        silence_output(session, 1);
        session->output_ready = ASIOOutputReady() == ASE_OK;
        session->accept_client.store(true, std::memory_order_release);
        session->last_callback_tick_ms.store(GetTickCount64(), std::memory_order_release);
        g_published.store(session, std::memory_order_release);
        const ASIOError result = ASIOStart();
        if (result != ASE_OK) {
            write_asio_error(error, error_len, "ASIOStart", result);
            session->accept_client.store(false, std::memory_order_release);
            Session *expected = session;
            g_published.compare_exchange_strong(expected, nullptr, std::memory_order_acq_rel);
            while (g_readers.load(std::memory_order_acquire) != 0) {
                SwitchToThread();
            }
            status = 16;
        } else {
            session->running.store(true, std::memory_order_release);
        }
    }

    if (status != 0) {
        cleanup_session(session, nullptr, 0);
        delete session;
        return status;
    }

    std::memset(out_started, 0, sizeof(*out_started));
    out_started->output_format = session->output_format;
    out_started->input_format = session->input_format;
    out_started->output_channels = session->output_channels;
    out_started->input_channels = session->input_channels;
    out_started->sample_rate_hz = session->sample_rate_hz;
    out_started->fixed_buffer_frames = session->frames;
    *out_session = session;
    return 0;
}

extern "C" int32_t sd_asio_v3_stop(void *opaque, char *error, size_t error_len) {
    Session *session = static_cast<Session *>(opaque);
    if (session == nullptr) {
        write_error(error, error_len, "ASIO v3 session is null");
        return 1;
    }
    return cleanup_session(session, error, error_len) ? 0 : 2;
}

extern "C" int32_t sd_asio_v3_close(void **opaque, char *error, size_t error_len) {
    if (opaque == nullptr || *opaque == nullptr) {
        write_error(error, error_len, "ASIO v3 session handle is null");
        return 1;
    }
    Session *session = static_cast<Session *>(*opaque);
    if (!cleanup_session(session, error, error_len)) {
        return 2;
    }
    delete session;
    *opaque = nullptr;
    return 0;
}

extern "C" void sd_asio_v3_telemetry(const void *opaque, SdAsioV3Telemetry *out_telemetry) {
    if (out_telemetry == nullptr) {
        return;
    }
    std::memset(out_telemetry, 0, sizeof(*out_telemetry));
    const Session *session = static_cast<const Session *>(opaque);
    if (session == nullptr) {
        return;
    }
    out_telemetry->callbacks = session->callbacks.load(std::memory_order_acquire);
    out_telemetry->xruns = session->xruns.load(std::memory_order_acquire);
    out_telemetry->last_callback_tick_ms =
        session->last_callback_tick_ms.load(std::memory_order_acquire);
    out_telemetry->terminal_kind = session->terminal_kind.load(std::memory_order_acquire);
    out_telemetry->running = session->running.load(std::memory_order_acquire) ? 1U : 0U;
}

extern "C" uint64_t sd_asio_v3_driver_open_attempts(void) {
    return g_driver_open_attempts.load(std::memory_order_acquire);
}

extern "C" uint32_t sd_asio_v3_has_published_session(void) {
    return g_published.load(std::memory_order_acquire) == nullptr ? 0U : 1U;
}

extern "C" void sd_asio_v3_latch_terminal(void *opaque, uint32_t terminal_kind) {
    latch_terminal(static_cast<Session *>(opaque), terminal_kind);
}
