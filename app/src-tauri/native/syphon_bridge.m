// Syndocal's narrow C ABI around the macOS-only Syphon/Metal output owner.
// Syphon.framework is built from the immutable source pin in prepare-syphon.sh.
#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#import <Syphon/SyphonMetalServer.h>
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
