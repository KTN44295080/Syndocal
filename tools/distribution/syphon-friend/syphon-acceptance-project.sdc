{
  "version": 2,
  "app": "Syndocal",
  "custom_profiles": [],
  "fixture_groups": [],
  "snapshot": {
    "fixtures": [],
    "cues": [],
    "active_cue_id": null,
    "active_fade": null,
    "timeline": {
      "id": 1,
      "label": "Syphon acceptance",
      "events": [],
      "automations": [],
      "video_automations": [],
      "audio": null,
      "playing": false,
      "position_ms": 0,
      "duration_ms": 60000
    },
    "video": {
      "layers": [
        {
          "id": 1,
          "label": "Syphon RGB corners + moving gold circle",
          "source": {
            "kind": {
              "Generator": {
                "Noise": {
                  "seed": 92,
                  "scale": 5,
                  "speed": 100,
                  "color_a": [
                    0,
                    0,
                    0,
                    255
                  ],
                  "color_b": [
                    255,
                    255,
                    255,
                    255
                  ]
                }
              }
            },
            "path": null,
            "name": "Media-free animated acceptance",
            "codec": null,
            "metadata": null
          },
          "blend_mode": "Normal",
          "state": {
            "enabled": true,
            "solo": false,
            "opacity": 1,
            "speed": 1,
            "playing": true,
            "position_ms": 0,
            "loop_enabled": true,
            "loop_start_ms": 0,
            "loop_end_ms": 60000,
            "bpm_sync": {
              "enabled": false,
              "ratio": 1,
              "loop_bars": 2
            },
            "cue_points": [],
            "cue_points_ms": [],
            "transform": {
              "x": 0,
              "y": 0,
              "scale_x": 1,
              "scale_y": 1,
              "rotation_deg": 0,
              "crop_left": 0,
              "crop_top": 0,
              "crop_right": 0,
              "crop_bottom": 0
            },
            "color": {
              "brightness": 0,
              "contrast": 1,
              "hue_deg": 0,
              "saturation": 1,
              "gamma": 1
            },
            "fx": {
              "pixelate": 1,
              "blur": 0,
              "glow": 0,
              "edge": 0,
              "key_red": 0,
              "key_green": 1,
              "key_blue": 0,
              "key_threshold": 0
            }
          },
          "isf_effect": {
            "enabled": true,
            "label": "Syphon RGB orientation + motion",
            "source": "/*{\"ISFVSN\":\"2\",\"DESCRIPTION\":\"Syndocal Syphon orientation, channel and motion acceptance\",\"INPUTS\":[]}*/\nfloat letter(vec2 local, float bits) {\n  vec2 cell = floor(local / 6.0);\n  if (cell.x < 0.0 || cell.x >= 3.0 || cell.y < 0.0 || cell.y >= 5.0) return 0.0;\n  return mod(floor(bits / pow(2.0, cell.y * 3.0 + cell.x)), 2.0);\n}\nvoid main() {\n  vec2 p = isf_FragNormCoord * vec2(1920.0, 1080.0);\n  vec3 c = vec3(0.08 + 0.04 * mod(floor(p.x / 80.0) + floor(p.y / 80.0), 2.0));\n  vec2 center = vec2(960.0 + 300.0 * sin(TIME), 540.0 + 150.0 * cos(TIME));\n  if (distance(p, center) < 80.0) c = vec3(1.0, 0.8, 0.0);\n  if (p.x < 620.0 && p.y < 150.0) c = vec3(1.0, 0.0, 0.0);\n  if (p.x >= 1300.0 && p.y < 150.0) c = vec3(0.0, 1.0, 0.0);\n  if (p.x < 620.0 && p.y >= 930.0) c = vec3(0.0, 0.0, 1.0);\n  float ink = 0.0;\n  ink = max(ink, letter(p - vec2(24.0, 24.0), 9367.0));\n  ink = max(ink, letter(p - vec2(48.0, 24.0), 31599.0));\n  ink = max(ink, letter(p - vec2(72.0, 24.0), 4843.0));\n  ink = max(ink, letter(p - vec2(96.0, 24.0), 448.0));\n  ink = max(ink, letter(p - vec2(120.0, 24.0), 29257.0));\n  ink = max(ink, letter(p - vec2(144.0, 24.0), 29391.0));\n  ink = max(ink, letter(p - vec2(168.0, 24.0), 4815.0));\n  ink = max(ink, letter(p - vec2(192.0, 24.0), 9367.0));\n  ink = max(ink, letter(p - vec2(216.0, 24.0), 0.0));\n  ink = max(ink, letter(p - vec2(240.0, 24.0), 8778.0));\n  ink = max(ink, letter(p - vec2(264.0, 24.0), 23275.0));\n  ink = max(ink, letter(p - vec2(288.0, 24.0), 29391.0));\n  ink = max(ink, letter(p - vec2(312.0, 24.0), 15211.0));\n  ink = max(ink, letter(p - vec2(336.0, 24.0), 10530.0));\n  ink = max(ink, letter(p - vec2(1324.0, 24.0), 9367.0));\n  ink = max(ink, letter(p - vec2(1348.0, 24.0), 31599.0));\n  ink = max(ink, letter(p - vec2(1372.0, 24.0), 4843.0));\n  ink = max(ink, letter(p - vec2(1396.0, 24.0), 448.0));\n  ink = max(ink, letter(p - vec2(1420.0, 24.0), 23275.0));\n  ink = max(ink, letter(p - vec2(1444.0, 24.0), 29847.0));\n  ink = max(ink, letter(p - vec2(1468.0, 24.0), 31567.0));\n  ink = max(ink, letter(p - vec2(1492.0, 24.0), 23533.0));\n  ink = max(ink, letter(p - vec2(1516.0, 24.0), 9367.0));\n  ink = max(ink, letter(p - vec2(1540.0, 24.0), 0.0));\n  ink = max(ink, letter(p - vec2(1564.0, 24.0), 8778.0));\n  ink = max(ink, letter(p - vec2(1588.0, 24.0), 31567.0));\n  ink = max(ink, letter(p - vec2(1612.0, 24.0), 23275.0));\n  ink = max(ink, letter(p - vec2(1636.0, 24.0), 29391.0));\n  ink = max(ink, letter(p - vec2(1660.0, 24.0), 29391.0));\n  ink = max(ink, letter(p - vec2(1684.0, 24.0), 24573.0));\n  ink = max(ink, letter(p - vec2(1708.0, 24.0), 10530.0));\n  ink = max(ink, letter(p - vec2(24.0, 954.0), 15083.0));\n  ink = max(ink, letter(p - vec2(48.0, 954.0), 31599.0));\n  ink = max(ink, letter(p - vec2(72.0, 954.0), 9367.0));\n  ink = max(ink, letter(p - vec2(96.0, 954.0), 9367.0));\n  ink = max(ink, letter(p - vec2(120.0, 954.0), 31599.0));\n  ink = max(ink, letter(p - vec2(144.0, 954.0), 23549.0));\n  ink = max(ink, letter(p - vec2(168.0, 954.0), 448.0));\n  ink = max(ink, letter(p - vec2(192.0, 954.0), 29257.0));\n  ink = max(ink, letter(p - vec2(216.0, 954.0), 29391.0));\n  ink = max(ink, letter(p - vec2(240.0, 954.0), 4815.0));\n  ink = max(ink, letter(p - vec2(264.0, 954.0), 9367.0));\n  ink = max(ink, letter(p - vec2(288.0, 954.0), 0.0));\n  ink = max(ink, letter(p - vec2(312.0, 954.0), 8778.0));\n  ink = max(ink, letter(p - vec2(336.0, 954.0), 15083.0));\n  ink = max(ink, letter(p - vec2(360.0, 954.0), 29257.0));\n  ink = max(ink, letter(p - vec2(384.0, 954.0), 31597.0));\n  ink = max(ink, letter(p - vec2(408.0, 954.0), 29391.0));\n  ink = max(ink, letter(p - vec2(432.0, 954.0), 10530.0));\n  gl_FragColor = vec4(mix(c, vec3(1.0), ink), 1.0);\n}\n",
            "controls": [],
            "categories": [
              "Acceptance"
            ]
          }
        }
      ],
      "compositions": [
        {
          "id": 1,
          "label": "Syphon 1920x1080",
          "layer_ids": [
            1
          ],
          "output_ids": [
            1
          ]
        }
      ],
      "outputs": [
        {
          "id": 1,
          "label": "Syndocal-Syphon-Test",
          "kind": "SyphonServer",
          "enabled": true,
          "composition_id": 1,
          "fullscreen": false,
          "monitor_id": null,
          "width": 1920,
          "height": 1080,
          "endpoint_name": "Syndocal-Syphon-Test",
          "opacity": 1,
          "blackout": false
        }
      ],
      "mapping_presets": [],
      "master_opacity": 1,
      "blackout": false
    },
    "effects": [],
    "node_graphs": [],
    "output": {
      "enabled": false,
      "protocol": "ArtNet",
      "target_ip": "127.0.0.1",
      "port": 6454,
      "universe": 0,
      "serial_port": "",
      "serial_baud_rate": 57600
    },
    "dmx_outputs": [],
    "lighting_master": 1,
    "submasters": [],
    "blackout": false,
    "clock": {
      "bpm": 120,
      "beat_phase": 0,
      "beat_counter": 0,
      "tap_count": 0,
      "source": "Manual"
    },
    "stage_map": {
      "locked": false,
      "min_x": -6,
      "max_x": 6,
      "min_z": -4,
      "max_z": 6
    },
    "stage_map_presets": [],
    "stage_objects": [],
    "dmx_preview": [],
    "dmx_previews": [],
    "telemetry": {
      "frame_counter": 0,
      "queue_depth": 0,
      "queue_depth_abs_max": 0,
      "queue_push_failure_count": 0,
      "last_tick_interval_us": 0,
      "tick_jitter_last_us": 0,
      "tick_jitter_abs_max_us": 0,
      "tick_jitter_stddev_us": 0,
      "tick_jitter_p95_us": 0,
      "tick_jitter_p99_us": 0,
      "tick_jitter_samples": 0,
      "last_command_queue_latency_us": 0,
      "command_queue_latency_abs_max_us": 0,
      "command_queue_latency_p95_us": 0,
      "command_queue_latency_p99_us": 0,
      "command_queue_latency_samples": 0,
      "last_command_drain_count": 0,
      "command_drain_abs_max": 0,
      "command_drain_limit_hit_count": 0,
      "last_command_to_dmx_tick_latency_us": 0,
      "command_to_dmx_tick_latency_abs_max_us": 0,
      "command_to_dmx_tick_latency_p95_us": 0,
      "command_to_dmx_tick_latency_p99_us": 0,
      "command_to_dmx_tick_latency_samples": 0,
      "last_dmx_send_interval_us": 0,
      "dmx_send_interval_min_us": 0,
      "dmx_send_interval_max_us": 0,
      "dmx_send_interval_samples": 0,
      "low_latency_dmx_tick_request_count": 0,
      "low_latency_dmx_tick_advance_count": 0,
      "low_latency_dmx_tick_defer_count": 0,
      "last_packet_bytes": 0,
      "last_dmx_output_count": 0,
      "last_dmx_send_success_count": 0,
      "last_dmx_send_failure_count": 0,
      "total_dmx_send_success_count": 0,
      "total_dmx_send_failure_count": 0,
      "last_dmx_route_results": [],
      "last_error": null
    }
  }
}
