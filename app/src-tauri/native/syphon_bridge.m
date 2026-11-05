// Syndocal's narrow C ABI around the macOS-only Syphon/Metal output owner.
// Syphon.framework is built from the immutable source pin in prepare-syphon.sh.
#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#import <Syphon/SyphonMetalServer.h>
#import <Syphon/SyphonMetalClient.h>
#import <Syphon/SyphonServerDirectory.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

@interface SyndocalSyphonOutput : NSObject
@property(nonatomic, strong) id<MTLDevice> device;
@property(nonatomic, strong) id<MTLCommandQueue> queue;
@property(nonatomic, strong) id<MTLTexture> texture;
@property(nonatomic, strong) id<MTLCommandBuffer> pending;
@property(nonatomic, strong) SyphonMetalServer *server;
@end

@implementation SyndocalSyphonOutput
@end

@interface SyndocalSyphonInput : NSObject
@property(nonatomic, strong) id<MTLDevice> device;
@property(nonatomic, strong) id<MTLCommandQueue> queue;
@property(nonatomic, strong) SyphonMetalClient *client;
@property(nonatomic, strong) id<MTLTexture> frame;
@property(nonatomic, strong) id<MTLBuffer> staging;
@property(nonatomic, strong) id<MTLCommandBuffer> pending;
@property(nonatomic) NSUInteger rowBytes;
@property(nonatomic) NSUInteger height;
@property(nonatomic, strong) dispatch_semaphore_t testFrameReady;
@end

@implementation SyndocalSyphonInput
@end

static void set_error(char *error, size_t capacity, const char *message) {
    if (error != NULL && capacity > 0) {
        snprintf(error, capacity, "%s", message);
    }
}

static void set_exception(char *error, size_t capacity, NSException *exception) {
    @try {
        const char *reason = exception.reason.UTF8String;
        set_error(error, capacity, reason != NULL ? reason : "Syphon Objective-C exception");
    } @catch (...) {
        set_error(error, capacity, "Syphon Objective-C exception while describing an error");
    }
}

// waitUntilCompleted waits for both GPU execution AND completion handlers.
// Syphon publishes from its completion handler, so waiting only for scheduling
// would let a frame escape after the physical output lease was retired.
static void drain_pending(SyndocalSyphonOutput *output) {
    if (output.pending != nil) {
        if (output.pending.status >= MTLCommandBufferStatusCommitted) {
            [output.pending waitUntilCompleted];
        }
        output.pending = nil;
    }
}

// Queried at construction only, never on the per-frame publication path.
// A missing handle is valid for recording a constructor failure.
int syndocal_syphon_metadata(void *handle, char *json, size_t capacity,
                            char *error, size_t error_capacity) {
    @try {
        @autoreleasepool {
            if (json == NULL || capacity == 0) {
                set_error(error, error_capacity, "Syphon metadata buffer is missing");
                return 0;
            }
            SyndocalSyphonOutput *output = handle != NULL
                ? (__bridge SyndocalSyphonOutput *)handle : nil;
            NSDictionary *metadata = @{
                @"macos_version": NSProcessInfo.processInfo.operatingSystemVersionString,
                @"metal_device_name": (id)output.device.name ?: NSNull.null,
                @"has_unified_memory": output.device != nil
                    ? (id)@(output.device.hasUnifiedMemory) : NSNull.null,
            };
            NSError *serializationError = nil;
            NSData *data = [NSJSONSerialization dataWithJSONObject:metadata options:0 error:&serializationError];
            if (data == nil || data.length >= capacity) {
                set_error(error, error_capacity, "Syphon metadata could not fit the bounded JSON buffer");
                return 0;
            }
            memcpy(json, data.bytes, data.length);
            json[data.length] = '\0';
            return 1;
        }
    } @catch (NSException *exception) {
        set_exception(error, error_capacity, exception);
        return 0;
    }
}

void *syndocal_syphon_create(const char *name, char *error, size_t capacity) {
    @try {
        @autoreleasepool {
            if (name == NULL) {
                set_error(error, capacity, "Syphon server name is missing");
                return NULL;
            }
            NSString *serverName = [NSString stringWithUTF8String:name];
            if (serverName == nil || serverName.length == 0) {
                set_error(error, capacity, "Syphon server name is not valid UTF-8");
                return NULL;
            }
            SyndocalSyphonOutput *output = [SyndocalSyphonOutput new];
            output.device = MTLCreateSystemDefaultDevice();
            if (output.device == nil) {
                set_error(error, capacity, "No Metal device is available for Syphon");
                return NULL;
            }
            output.queue = [output.device newCommandQueue];
            if (output.queue == nil) {
                set_error(error, capacity, "Could not create the Syphon Metal command queue");
                return NULL;
            }
            // Init announces the server name; it does not publish a frame. The
            // Rust worker's readiness/Arm barrier gates every call to send.
            output.server = [[SyphonMetalServer alloc] initWithName:serverName
                                                           device:output.device
                                                          options:nil];
            if (output.server == nil) {
                set_error(error, capacity, "Could not start the Syphon server");
                return NULL;
            }
            return (__bridge_retained void *)output;
        }
    } @catch (NSException *exception) {
        set_exception(error, capacity, exception);
        return NULL;
    }
}

int syndocal_syphon_send_rgba(void *handle, uint32_t width, uint32_t height,
                             const uint8_t *rgba, size_t length,
                             char *error, size_t capacity) {
    @try {
        @autoreleasepool {
            if (handle == NULL || rgba == NULL || width == 0 || height == 0
                || width > 8192 || height > 8192
                || length != (size_t)width * (size_t)height * 4) {
                set_error(error, capacity, "Syphon frame must contain tightly packed RGBA8 pixels (1..8192 per side)");
                return 0;
            }
            SyndocalSyphonOutput *output = (__bridge SyndocalSyphonOutput *)handle;
            if (output.server == nil) {
                set_error(error, capacity, "Syphon server is closed");
                return 0;
            }
            drain_pending(output);
            if (output.texture == nil || output.texture.width != width || output.texture.height != height) {
                MTLTextureDescriptor *descriptor = [MTLTextureDescriptor
                    texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
                                                width:width height:height mipmapped:NO];
                descriptor.usage = MTLTextureUsageShaderRead;
                descriptor.storageMode = output.device.hasUnifiedMemory
                    ? MTLStorageModeShared : MTLStorageModeManaged;
                output.texture = [output.device newTextureWithDescriptor:descriptor];
                if (output.texture == nil) {
                    set_error(error, capacity, "Could not allocate the Syphon upload texture");
                    return 0;
                }
            }
            [output.texture replaceRegion:MTLRegionMake2D(0, 0, width, height)
                              mipmapLevel:0 withBytes:rgba bytesPerRow:(NSUInteger)width * 4];
            output.pending = [output.queue commandBuffer];
            if (output.pending == nil) {
                set_error(error, capacity, "Could not allocate the Syphon Metal command buffer");
                return 0;
            }
            // The engine's RGBA bytes are top-row-first. Syphon's shader handles
            // RGBA -> BGRA conversion while applying its vertical convention.
            [output.server publishFrameTexture:output.texture
                               onCommandBuffer:output.pending
                                   imageRegion:NSMakeRect(0, 0, width, height)
                                       flipped:NO];
            // The upstream API returns void and can skip encoding if its
            // IOSurface allocation fails. Do not report an empty GPU submission
            // as a delivered frame. ARC releases this +1 image reference.
            id<MTLTexture> published = [output.server newFrameImage];
            if (published == nil || published.width != width || published.height != height) {
                set_error(error, capacity, "Syphon could not allocate its published IOSurface texture");
                return 0;
            }
            [output.pending commit];
            [output.pending waitUntilCompleted];
            if (output.pending.status == MTLCommandBufferStatusError) {
                const char *reason = output.pending.error.localizedDescription.UTF8String;
                set_error(error, capacity, reason != NULL ? reason : "Syphon Metal command failed");
                output.pending = nil;
                return 0;
            }
            output.pending = nil;
            return 1;
        }
    } @catch (NSException *exception) {
        // Keep the handle and any submitted buffer owned. close() must drain it
        // before the Rust physical-output lease can report retirement.
        set_exception(error, capacity, exception);
        return 0;
    }
}

// Explicit CI probe, never called by the live renderer: read the *published*
// IOSurface texture back and verify a tiny asymmetric RGBA test pattern.
int syndocal_syphon_verify_test_pattern(void *handle, const uint8_t *rgba,
                                       char *error, size_t capacity) {
    @try {
        @autoreleasepool {
            if (handle == NULL || rgba == NULL) {
                set_error(error, capacity, "Syphon readback probe is missing its owner or pattern");
                return 0;
            }
            SyndocalSyphonOutput *output = (__bridge SyndocalSyphonOutput *)handle;
            drain_pending(output);
            id<MTLTexture> texture = [output.server newFrameImage];
            if (texture == nil || texture.width != 2 || texture.height != 2
                || texture.pixelFormat != MTLPixelFormatBGRA8Unorm) {
                set_error(error, capacity, "Syphon readback probe requires the published 2x2 BGRA IOSurface");
                return 0;
            }
            const NSUInteger rowBytes = 256;
            id<MTLBuffer> buffer = [output.device newBufferWithLength:rowBytes * 2 options:MTLResourceStorageModeShared];
            id<MTLCommandBuffer> command = [output.queue commandBuffer];
            id<MTLBlitCommandEncoder> blit = [command blitCommandEncoder];
            if (buffer == nil || command == nil || blit == nil) {
                set_error(error, capacity, "Could not allocate the Syphon readback probe");
                return 0;
            }
            [blit copyFromTexture:texture sourceSlice:0 sourceLevel:0
                    sourceOrigin:MTLOriginMake(0, 0, 0) sourceSize:MTLSizeMake(2, 2, 1)
                        toBuffer:buffer destinationOffset:0 destinationBytesPerRow:rowBytes
                destinationBytesPerImage:rowBytes * 2];
            [blit endEncoding];
            [command commit];
            [command waitUntilCompleted];
            if (command.status == MTLCommandBufferStatusError) {
                set_error(error, capacity, "Syphon readback probe command failed");
                return 0;
            }
            const uint8_t *bgra = buffer.contents;
            for (NSUInteger y = 0; y < 2; y++) {
                for (NSUInteger x = 0; x < 2; x++) {
                    const uint8_t *expected = rgba + (y * 2 + x) * 4;
                    const uint8_t *actual = bgra + y * rowBytes + x * 4;
                    if (actual[2] != expected[0] || actual[1] != expected[1]
                        || actual[0] != expected[2] || actual[3] != expected[3]) {
                        set_error(error, capacity, "Syphon published IOSurface has incorrect channel order or orientation");
                        return 0;
                    }
                }
            }
            return 1;
        }
    } @catch (NSException *exception) {
        set_exception(error, capacity, exception);
        return 0;
    }
}

int syndocal_syphon_close(void *handle, char *error, size_t capacity) {
    @try {
        @autoreleasepool {
            if (handle == NULL) {
                return 1;
            }
            SyndocalSyphonOutput *output = (__bridge SyndocalSyphonOutput *)handle;
            drain_pending(output);
            [output.server stop];
            output.server = nil;
            output.texture = nil;
            output.queue = nil;
            output.device = nil;
            // Transfer the retained ownership only AFTER all shutdown calls
            // succeeded. A failed close preserves a valid handle for retry.
            (void)CFBridgingRelease(handle);
            return 1;
        }
    } @catch (NSException *exception) {
        set_exception(error, capacity, exception);
        return 0;
    }
}

// Discovery/selection runs only when requested by the UI or constructing a
// decoder client. Never serialize a framework dictionary (it contains icons
// and private transport data) across the ABI.
static BOOL bounded_server_string(id value, NSUInteger maximum) {
    return [value isKindOfClass:NSString.class]
        && [(NSString *)value lengthOfBytesUsingEncoding:NSUTF8StringEncoding] <= maximum
        && [(NSString *)value rangeOfCharacterFromSet:NSCharacterSet.controlCharacterSet].location == NSNotFound;
}

int syndocal_syphon_list_servers(char *json, size_t capacity,
                                char *error, size_t error_capacity) {
    @try {
        @autoreleasepool {
            if (json == NULL || capacity == 0) {
                set_error(error, error_capacity, "Syphon server list buffer is missing");
                return 0;
            }
            NSArray *servers = SyphonServerDirectory.sharedDirectory.servers;
            if (servers.count > 64) {
                set_error(error, error_capacity, "Syphon discovery exceeds 64 servers; close unused publishers and refresh");
                return 0;
            }
            NSMutableArray *result = [NSMutableArray arrayWithCapacity:servers.count];
            for (NSDictionary *server in servers) {
                NSString *uuid = server[SyphonServerDescriptionUUIDKey];
                NSString *name = server[SyphonServerDescriptionNameKey] ?: @"";
                NSString *app = server[SyphonServerDescriptionAppNameKey] ?: @"";
                if (!bounded_server_string(uuid, 1024) || uuid.length == 0
                    || !bounded_server_string(name, 1024) || !bounded_server_string(app, 1024) || app.length == 0) {
                    set_error(error, error_capacity, "Syphon publisher has an invalid or oversized identity; close it and refresh");
                    return 0;
                }
                [result addObject:@{@"uuid": uuid, @"name": name, @"app_name": app}];
            }
            NSData *data = [NSJSONSerialization dataWithJSONObject:result options:0 error:NULL];
            if (data == nil || data.length >= capacity) {
                set_error(error, error_capacity, "Syphon server list exceeds the bounded JSON buffer");
                return 0;
            }
            memcpy(json, data.bytes, data.length);
            json[data.length] = '\0';
            return 1;
        }
    } @catch (NSException *exception) {
        set_exception(error, error_capacity, exception);
        return 0;
    }
}

static void *create_syphon_client(NSDictionary *description, dispatch_semaphore_t frameReady,
                                 char *error, size_t capacity) {
    SyndocalSyphonInput *input = [SyndocalSyphonInput new];
    input.device = MTLCreateSystemDefaultDevice();
    if (input.device == nil) {
        set_error(error, capacity, "No Metal device is available for Syphon input");
        return NULL;
    }
    input.queue = [input.device newCommandQueue];
    if (input.queue == nil) {
        set_error(error, capacity, "Could not create the Syphon input Metal command queue");
        return NULL;
    }
    input.testFrameReady = frameReady;
    // Production uses on-demand consumption; only the deterministic in-process
    // CI test supplies a semaphore handler. It never captures its own client.
    void (^handler)(SyphonMetalClient *) = nil;
    if (frameReady != nil) {
        handler = ^(SyphonMetalClient *client) {
            (void)client;
            dispatch_semaphore_signal(frameReady);
        };
    }
    input.client = [[SyphonMetalClient alloc] initWithServerDescription:description
        device:input.device options:nil newFrameHandler:handler];
    if (input.client == nil || !input.client.isValid) {
        [input.client stop];
        set_error(error, capacity, "Could not connect to the selected Syphon publisher; start it and refresh the server list");
        return NULL;
    }
    return (__bridge_retained void *)input;
}

void *syndocal_syphon_client_create(const char *endpoint, char *error, size_t capacity) {
    @try {
        @autoreleasepool {
            if (endpoint == NULL || strnlen(endpoint, 4097) > 4096) {
                set_error(error, capacity, "Syphon input requires a bounded discovered server identity; refresh and select a server");
                return NULL;
            }
            NSData *data = [[NSString stringWithUTF8String:endpoint] dataUsingEncoding:NSUTF8StringEncoding];
            id identity = data != nil ? [NSJSONSerialization JSONObjectWithData:data options:0 error:NULL] : nil;
            if (![identity isKindOfClass:NSDictionary.class]
                || !bounded_server_string(identity[@"uuid"], 1024) || [identity[@"uuid"] length] == 0
                || !bounded_server_string(identity[@"name"], 1024)
                || !bounded_server_string(identity[@"app_name"], 1024) || [identity[@"app_name"] length] == 0
                || [identity count] != 3) {
                set_error(error, capacity, "Invalid Syphon input identity; refresh and select a discovered server");
                return NULL;
            }
            NSArray *servers = SyphonServerDirectory.sharedDirectory.servers;
            if (servers.count > 64) {
                set_error(error, capacity, "Syphon discovery exceeds 64 servers; close unused publishers");
                return NULL;
            }
            NSDictionary *selected = nil;
            NSUInteger matches = 0;
            for (NSDictionary *server in servers) {
                if ([server[SyphonServerDescriptionUUIDKey] isEqual:identity[@"uuid"]]) {
                    selected = server;
                    matches++;
                }
            }
            // UUIDs identify one live instance. A restarted publisher receives
            // a new UUID; recover only through an unambiguous exact app/name.
            if (matches == 0 && ([identity[@"name"] length] != 0 || [identity[@"app_name"] length] != 0)) {
                for (NSDictionary *server in servers) {
                    if ([(server[SyphonServerDescriptionNameKey] ?: @"") isEqual:identity[@"name"]]
                        && [(server[SyphonServerDescriptionAppNameKey] ?: @"") isEqual:identity[@"app_name"]]) {
                        selected = server;
                        matches++;
                    }
                }
            }
            if (matches != 1) {
                set_error(error, capacity, matches == 0
                    ? "Selected Syphon server is missing; start the publisher or refresh and select another server"
                    : "Selected Syphon server is ambiguous; give publishers distinct names and select again");
                return NULL;
            }
            return create_syphon_client(selected, nil, error, capacity);
        }
    } @catch (NSException *exception) {
        set_exception(error, capacity, exception);
        return NULL;
    }
}

// 0 = connected/no new image, 1 = packed pixels, 2 = resize the caller's buffer,
// -1 = actionable error. Keep a +1 frame across the resize handshake so a size
// query cannot consume the only new-frame notification without returning it.
int syndocal_syphon_client_try_receive_rgba(void *handle, uint32_t *width, uint32_t *height,
                                          uint8_t *rgba, size_t rgba_capacity,
                                          char *error, size_t error_capacity) {
    @try {
        @autoreleasepool {
            if (handle == NULL || width == NULL || height == NULL) {
                set_error(error, error_capacity, "Syphon input is closed or its dimension buffer is missing");
                return -1;
            }
            SyndocalSyphonInput *input = (__bridge SyndocalSyphonInput *)handle;
            if (input.client == nil || !input.client.isValid) {
                input.frame = nil;
                set_error(error, error_capacity, "Selected Syphon server vanished; restart its publisher or refresh the server selection");
                return -1;
            }
            if (input.frame == nil) {
                if (!input.client.hasNewFrame) return 0;
                input.frame = [input.client newFrameImage];
                if (input.frame == nil) {
                    set_error(error, error_capacity, "Syphon publisher announced a frame without an image; check the publisher");
                    return -1;
                }
            }
            id<MTLTexture> frame = input.frame;
            if (frame.width == 0 || frame.height == 0 || frame.width > 8192 || frame.height > 8192
                || (frame.pixelFormat != MTLPixelFormatBGRA8Unorm && frame.pixelFormat != MTLPixelFormatRGBA8Unorm)) {
                input.frame = nil;
                set_error(error, error_capacity, "Syphon input requires BGRA8/RGBA8 pixels with dimensions 1..8192");
                return -1;
            }
            *width = (uint32_t)frame.width;
            *height = (uint32_t)frame.height;
            if (rgba == NULL || rgba_capacity < frame.width * frame.height * 4) return 2;
            NSUInteger rowBytes = (frame.width * 4 + 255) & ~(NSUInteger)255;
            if (input.staging == nil || input.rowBytes != rowBytes || input.height != frame.height) {
                input.staging = [input.device newBufferWithLength:rowBytes * frame.height options:MTLResourceStorageModeShared];
                input.rowBytes = rowBytes;
                input.height = frame.height;
                if (input.staging == nil) {
                    set_error(error, error_capacity, "Could not allocate bounded Syphon input staging storage");
                    return -1;
                }
            }
            input.pending = [input.queue commandBuffer];
            id<MTLBlitCommandEncoder> blit = [input.pending blitCommandEncoder];
            if (input.pending == nil || blit == nil) {
                set_error(error, error_capacity, "Could not encode Syphon input readback");
                return -1;
            }
            [blit copyFromTexture:frame sourceSlice:0 sourceLevel:0
                sourceOrigin:MTLOriginMake(0, 0, 0) sourceSize:MTLSizeMake(frame.width, frame.height, 1)
                toBuffer:input.staging destinationOffset:0 destinationBytesPerRow:rowBytes
                destinationBytesPerImage:rowBytes * frame.height];
            [blit endEncoding];
            [input.pending commit];
            [input.pending waitUntilCompleted];
            if (input.pending.status == MTLCommandBufferStatusError) {
                set_error(error, error_capacity, "Syphon input GPU readback failed; check the publisher and Metal device");
                input.pending = nil;
                input.frame = nil;
                return -1;
            }
            input.pending = nil;
            const uint8_t *source = input.staging.contents;
            // Syphon's publishing API normalizes its flipped flag into the
            // shared image. Metal row zero is the engine's top row: no second
            // flip here. The in-process RGB-corner loopback pins this contract.
            for (NSUInteger y = 0; y < frame.height; y++) {
                const uint8_t *row = source + y * rowBytes;
                uint8_t *destination = rgba + y * frame.width * 4;
                if (frame.pixelFormat == MTLPixelFormatRGBA8Unorm) {
                    memcpy(destination, row, frame.width * 4);
                } else {
                    for (NSUInteger x = 0; x < frame.width; x++) {
                        destination[x * 4] = row[x * 4 + 2];
                        destination[x * 4 + 1] = row[x * 4 + 1];
                        destination[x * 4 + 2] = row[x * 4];
                        destination[x * 4 + 3] = row[x * 4 + 3];
                    }
                }
            }
            input.frame = nil;
            return 1;
        }
    } @catch (NSException *exception) {
        set_exception(error, error_capacity, exception);
        return -1;
    }
}

int syndocal_syphon_client_close(void *handle, char *error, size_t capacity) {
    @try {
        @autoreleasepool {
            if (handle == NULL) return 1;
            SyndocalSyphonInput *input = (__bridge SyndocalSyphonInput *)handle;
            if (input.pending != nil && input.pending.status >= MTLCommandBufferStatusCommitted) {
                [input.pending waitUntilCompleted];
            }
            input.pending = nil;
            [input.client stop];
            input.client = nil;
            input.frame = nil;
            input.staging = nil;
            input.queue = nil;
            input.device = nil;
            input.testFrameReady = nil;
            (void)CFBridgingRelease(handle);
            return 1;
        }
    } @catch (NSException *exception) {
        // A failed close never consumes the retained owner; Rust keeps it.
        set_exception(error, capacity, exception);
        return 0;
    }
}

// Called only by hosted-macOS tests. A direct description avoids asynchronous
// system-wide discovery, so this proves the actual client transport in-process.
void *syndocal_syphon_test_client_create(void *server, char *error, size_t capacity) {
    @try {
        @autoreleasepool {
            if (server == NULL) return NULL;
            SyndocalSyphonOutput *output = (__bridge SyndocalSyphonOutput *)server;
            return create_syphon_client(output.server.serverDescription,
                dispatch_semaphore_create(0), error, capacity);
        }
    } @catch (NSException *exception) {
        set_exception(error, capacity, exception);
        return NULL;
    }
}

int syndocal_syphon_test_wait_frame(void *handle) {
    @try {
        SyndocalSyphonInput *input = (__bridge SyndocalSyphonInput *)handle;
        if (input == nil || input.testFrameReady == nil) return 0;
        return dispatch_semaphore_wait(input.testFrameReady,
            dispatch_time(DISPATCH_TIME_NOW, 5 * NSEC_PER_SEC)) == 0;
    } @catch (...) {
        return 0;
    }
}

static __weak SyphonMetalServer *testServer;
static __weak SyphonMetalClient *testClient;

int syndocal_syphon_test_lifecycle(void *server, void *client) {
    @try {
        if (server != NULL && client != NULL) {
            testServer = ((__bridge SyndocalSyphonOutput *)server).server;
            testClient = ((__bridge SyndocalSyphonInput *)client).client;
            return testServer != nil && testClient != nil;
        }
        return testServer == nil && testClient == nil;
    } @catch (...) {
        return 0;
    }
}
