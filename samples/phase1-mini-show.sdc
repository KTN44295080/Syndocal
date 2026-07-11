{
  "version": 1,
  "app": "Syndocal",
  "custom_profiles": [
    {
      "source_path": "memory://custom/Syndocal-Phase_1_Mini_Spot",
      "manufacturer": "Syndocal",
      "name": "Phase 1 Mini Spot",
      "short_name": "P1 Spot",
      "fixture_type_id": null,
      "dmx_modes": [
        {
          "name": "8ch",
          "controls": [
            {
              "attribute": "Dimmer",
              "channel_name": "Dimmer",
              "geometry": "Beam",
              "offsets": [1],
              "resolution": "EightBit",
              "default_value": 0,
              "functions": []
            },
            {
              "attribute": "ColorRed",
              "channel_name": "ColorRed",
              "geometry": "Beam",
              "offsets": [2],
              "resolution": "EightBit",
              "default_value": 0,
              "functions": []
            },
            {
              "attribute": "ColorGreen",
              "channel_name": "ColorGreen",
              "geometry": "Beam",
              "offsets": [3],
              "resolution": "EightBit",
              "default_value": 0,
              "functions": []
            },
            {
              "attribute": "ColorBlue",
              "channel_name": "ColorBlue",
              "geometry": "Beam",
              "offsets": [4],
              "resolution": "EightBit",
              "default_value": 0,
              "functions": []
            },
            {
              "attribute": "Pan",
              "channel_name": "Pan",
              "geometry": "Head",
              "offsets": [5, 6],
              "resolution": "SixteenBit",
              "default_value": 32768,
              "functions": []
            },
            {
              "attribute": "Tilt",
              "channel_name": "Tilt",
              "geometry": "Head",
              "offsets": [7, 8],
              "resolution": "SixteenBit",
              "default_value": 32768,
              "functions": []
            }
          ]
        }
      ],
      "geometries": [
        {
          "name": "Body",
          "kind": "Geometry",
          "parent": null,
          "matrix": [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
          "model_name": "Mini Spot Body",
          "model_file": null,
          "model_primitive": "Cylinder",
          "model_dimensions": { "x": 0.42, "y": 0.32, "z": 0.42 },
          "beam_type": null,
          "beam_angle_deg": null,
          "field_angle_deg": null,
          "beam_radius": null
        },
        {
          "name": "Head",
          "kind": "Axis",
          "parent": "Body",
          "matrix": [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.34, 0.0, 0.0, 1.0, -0.18, 0.0, 0.0, 0.0, 1.0],
          "model_name": "Mini Spot Head",
          "model_file": null,
          "model_primitive": "Sphere",
          "model_dimensions": { "x": 0.34, "y": 0.28, "z": 0.34 },
          "beam_type": null,
          "beam_angle_deg": null,
          "field_angle_deg": null,
          "beam_radius": null
        },
        {
          "name": "Beam",
          "kind": "Beam",
          "parent": "Head",
          "matrix": [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.1, 0.0, 0.0, 1.0, -0.44, 0.0, 0.0, 0.0, 1.0],
          "model_name": "Mini Spot Beam",
          "model_file": null,
          "model_primitive": "Plane",
          "model_dimensions": { "x": 0.18, "y": 0.02, "z": 0.58 },
          "beam_type": "Spot",
          "beam_angle_deg": 18.0,
          "field_angle_deg": 28.0,
          "beam_radius": 0.09
        }
      ],
      "warnings": []
    }
  ],
  "snapshot": {
    "fixtures": [
      {
        "id": 1,
        "label": "Mini Spot 1",
        "profile_source_path": "memory://custom/Syndocal-Phase_1_Mini_Spot",
        "profile_name": "Phase 1 Mini Spot",
        "manufacturer": "Syndocal",
        "mode_name": "8ch",
        "universe": 0,
        "address": 1,
        "group_ids": ["Front"],
        "position": {
          "x": -2.0,
          "y": 3.0,
          "z": 0.0
        },
        "rotation": {
          "pitch": 0.0,
          "yaw": 0.0,
          "roll": 0.0
        },
        "geometries": [
          {
            "name": "Body",
            "kind": "Geometry",
            "parent": null,
            "matrix": [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            "model_name": "Mini Spot Body",
            "model_file": null,
            "model_primitive": "Cylinder",
            "model_dimensions": { "x": 0.42, "y": 0.32, "z": 0.42 },
            "beam_type": null,
            "beam_angle_deg": null,
            "field_angle_deg": null,
            "beam_radius": null
          },
          {
            "name": "Head",
            "kind": "Axis",
            "parent": "Body",
            "matrix": [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.34, 0.0, 0.0, 1.0, -0.18, 0.0, 0.0, 0.0, 1.0],
            "model_name": "Mini Spot Head",
            "model_file": null,
            "model_primitive": "Sphere",
            "model_dimensions": { "x": 0.34, "y": 0.28, "z": 0.34 },
            "beam_type": null,
            "beam_angle_deg": null,
            "field_angle_deg": null,
            "beam_radius": null
          },
          {
            "name": "Beam",
            "kind": "Beam",
            "parent": "Head",
            "matrix": [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.1, 0.0, 0.0, 1.0, -0.44, 0.0, 0.0, 0.0, 1.0],
            "model_name": "Mini Spot Beam",
            "model_file": null,
            "model_primitive": "Plane",
            "model_dimensions": { "x": 0.18, "y": 0.02, "z": 0.58 },
            "beam_type": "Spot",
            "beam_angle_deg": 18.0,
            "field_angle_deg": 28.0,
            "beam_radius": 0.09
          }
        ],
        "controls": [
          {
            "attribute": "Dimmer",
            "channel_name": "Dimmer",
            "geometry": "Beam",
            "offsets": [1],
            "resolution": "EightBit",
            "default_value": 0,
            "functions": []
          },
          {
            "attribute": "ColorRed",
            "channel_name": "ColorRed",
            "geometry": "Beam",
            "offsets": [2],
            "resolution": "EightBit",
            "default_value": 0,
            "functions": []
          },
          {
            "attribute": "ColorGreen",
            "channel_name": "ColorGreen",
            "geometry": "Beam",
            "offsets": [3],
            "resolution": "EightBit",
            "default_value": 0,
            "functions": []
          },
          {
            "attribute": "ColorBlue",
            "channel_name": "ColorBlue",
            "geometry": "Beam",
            "offsets": [4],
            "resolution": "EightBit",
            "default_value": 0,
            "functions": []
          },
          {
            "attribute": "Pan",
            "channel_name": "Pan",
            "geometry": "Head",
            "offsets": [5, 6],
            "resolution": "SixteenBit",
            "default_value": 32768,
            "functions": []
          },
          {
            "attribute": "Tilt",
            "channel_name": "Tilt",
            "geometry": "Head",
            "offsets": [7, 8],
            "resolution": "SixteenBit",
            "default_value": 32768,
            "functions": []
          }
        ],
        "attribute_values": [
          {
            "attribute": "Dimmer",
            "value": 0
          },
          {
            "attribute": "ColorRed",
            "value": 0
          },
          {
            "attribute": "ColorGreen",
            "value": 0
          },
          {
            "attribute": "ColorBlue",
            "value": 0
          },
          {
            "attribute": "Pan",
            "value": 32768
          },
          {
            "attribute": "Tilt",
            "value": 32768
          }
        ],
        "limits": {
          "dimmer_min": 0,
          "dimmer_max": 65535,
          "pan_min": 0,
          "pan_max": 65535,
          "tilt_min": 0,
          "tilt_max": 65535,
          "invert_pan": false,
          "invert_tilt": false,
          "swap_pan_tilt": false
        },
        "highlighted": false,
        "soloed": false,
        "parked": false
      }
    ],
    "cues": [
      {
        "id": 1,
        "label": "White Hit",
        "fade_ms": 100,
        "targets": [
          {
            "fixture_id": 1,
            "values": [
              {
                "attribute": "Dimmer",
                "value": 65535
              },
              {
                "attribute": "ColorRed",
                "value": 65535
              },
              {
                "attribute": "ColorGreen",
                "value": 65535
              },
              {
                "attribute": "ColorBlue",
                "value": 65535
              }
            ]
          }
        ],
        "video_targets": [
          {
            "layer_id": 1,
            "state": {
              "enabled": true,
              "solo": false,
              "opacity": 1.0,
              "speed": 1.0,
              "playing": true,
              "position_ms": 0,
              "loop_enabled": true,
              "loop_start_ms": 0,
              "loop_end_ms": 4000,
              "bpm_sync": {
                "enabled": true,
                "ratio": 1.0,
                "loop_bars": 2.0
              },
              "cue_points": [
                {
                  "position_ms": 0,
                  "label": "Start",
                  "color": "#5dd64c"
                },
                {
                  "position_ms": 2000,
                  "label": "Drop",
                  "color": "#ff2b88"
                }
              ],
              "cue_points_ms": [0, 2000],
              "transform": {
                "x": 0.0,
                "y": 0.0,
                "scale_x": 1.0,
                "scale_y": 1.0,
                "rotation_deg": 0.0,
                "crop_left": 0.0,
                "crop_top": 0.0,
                "crop_right": 0.0,
                "crop_bottom": 0.0
              },
              "color": {
                "brightness": 0.0,
                "contrast": 1.0,
                "hue_deg": 0.0,
                "saturation": 1.0,
                "gamma": 1.0
              },
              "fx": {
                "pixelate": 1.0,
                "blur": 0.0,
                "glow": 0.0,
                "edge": 0.0,
                "key_red": 0.0,
                "key_green": 1.0,
                "key_blue": 0.0,
                "key_threshold": 0.0
              }
            }
          }
        ],
        "video_output_targets": [
          {
            "output_id": 1,
            "enabled": true,
            "opacity": 1.0,
            "blackout": false
          }
        ],
        "node_graph_targets": []
      }
    ],
    "active_cue_id": null,
    "active_fade": null,
    "timeline": {
      "events": [
        {
          "id": 1,
          "cue_id": 1,
          "time_ms": 0,
          "track": "Lighting"
        },
        {
          "id": 2,
          "cue_id": 1,
          "time_ms": 4000,
          "track": "Lighting"
        }
      ],
      "automations": [
        {
          "id": 1,
          "fixture_id": 1,
          "attribute": "Dimmer",
          "track": "Lighting",
          "keyframes": [
            {
              "time_ms": 0,
              "value": 65535,
              "interpolation": "Linear"
            },
            {
              "time_ms": 4000,
              "value": 0,
              "interpolation": "Linear"
            }
          ],
          "enabled": true
        }
      ],
      "video_automations": [
        {
          "id": 2,
          "layer_id": 1,
          "param": "Opacity",
          "track": "Video",
          "keyframes": [
            {
              "time_ms": 0,
              "value": 1.0,
              "interpolation": "Linear"
            },
            {
              "time_ms": 4000,
              "value": 0.0,
              "interpolation": "Linear"
            }
          ],
          "enabled": true
        }
      ],
      "audio": null,
      "playing": false,
      "position_ms": 0,
      "duration_ms": 4000
    },
    "video": {
      "layers": [
        {
          "id": 1,
          "label": "Stage NDI",
          "source": {
            "kind": "Ndi",
            "path": null,
            "name": "Stage NDI",
            "codec": null,
            "metadata": null
          },
          "blend_mode": "Normal",
          "state": {
            "enabled": true,
            "solo": false,
            "opacity": 0.7,
            "speed": 1.0,
            "playing": false,
            "position_ms": 0,
            "loop_enabled": true,
            "loop_start_ms": 0,
            "loop_end_ms": 4000,
            "bpm_sync": {
              "enabled": true,
              "ratio": 1.0,
              "loop_bars": 2.0
            },
            "cue_points": [
              {
                "position_ms": 0,
                "label": "Start",
                "color": "#5dd64c"
              },
              {
                "position_ms": 2000,
                "label": "Drop",
                "color": "#ff2b88"
              }
            ],
            "cue_points_ms": [0, 2000],
            "transform": {
              "x": 0.0,
              "y": 0.0,
              "scale_x": 1.0,
              "scale_y": 1.0,
              "rotation_deg": 0.0,
              "crop_left": 0.0,
              "crop_top": 0.0,
              "crop_right": 0.0,
              "crop_bottom": 0.0
            },
            "color": {
              "brightness": 0.0,
              "contrast": 1.0,
              "hue_deg": 0.0,
              "saturation": 1.0,
              "gamma": 1.0
            },
            "fx": {
              "pixelate": 1.0,
              "blur": 0.0,
              "glow": 0.0,
              "edge": 0.0,
              "key_red": 0.0,
              "key_green": 1.0,
              "key_blue": 0.0,
              "key_threshold": 0.0
            }
          }
        }
      ],
      "compositions": [
        {
          "id": 1,
          "label": "Main",
          "layer_ids": [1],
          "output_ids": [1]
        }
      ],
      "outputs": [
        {
          "id": 1,
          "label": "Front Projector",
          "kind": "Display",
          "enabled": true,
          "composition_id": 1,
          "fullscreen": false,
          "monitor_id": 0,
          "width": 1920,
          "height": 1080,
          "endpoint_name": null,
          "opacity": 1.0,
          "blackout": false,
          "mapping": {
            "stage_x": 0.0,
            "stage_y": 0.0,
            "stage_z": 4.0,
            "offset_x": 0.0,
            "offset_y": 0.0,
            "scale_x": 1.0,
            "scale_y": 1.0,
            "rotation_deg": 0.0,
            "aspect_ratio": 1.7777778,
            "aspect_mode": "Fit",
            "lens_distortion": 0.0,
            "keystone_x": 0.08,
            "keystone_y": -0.04,
            "corner_top_left_x": -0.04,
            "corner_top_left_y": 0.02,
            "corner_top_right_x": 0.03,
            "corner_top_right_y": 0.01,
            "corner_bottom_right_x": 0.02,
            "corner_bottom_right_y": -0.03,
            "corner_bottom_left_x": -0.03,
            "corner_bottom_left_y": -0.02
          }
        }
      ],
      "mapping_presets": [
        {
          "label": "16:9 Front Fit",
          "mapping": {
            "stage_x": 0.0,
            "stage_y": 0.0,
            "stage_z": 4.0,
            "offset_x": 0.0,
            "offset_y": 0.0,
            "scale_x": 1.0,
            "scale_y": 1.0,
            "rotation_deg": 0.0,
            "aspect_ratio": 1.7777778,
            "aspect_mode": "Fit",
            "lens_distortion": 0.0,
            "keystone_x": 0.08,
            "keystone_y": -0.04,
            "corner_top_left_x": -0.04,
            "corner_top_left_y": 0.02,
            "corner_top_right_x": 0.03,
            "corner_top_right_y": 0.01,
            "corner_bottom_right_x": 0.02,
            "corner_bottom_right_y": -0.03,
            "corner_bottom_left_x": -0.03,
            "corner_bottom_left_y": -0.02
          }
        }
      ],
      "master_opacity": 1.0,
      "blackout": false
    },
    "effects": [],
    "node_graphs": [],
    "output": {
      "enabled": true,
      "protocol": "ArtNet",
      "target_ip": "127.0.0.1",
      "port": 6454,
      "universe": 0,
      "serial_port": "",
      "serial_baud_rate": 57600
    },
    "dmx_outputs": [
      {
        "enabled": true,
        "protocol": "ArtNet",
        "target_ip": "127.0.0.1",
        "port": 6454,
        "universe": 0,
        "serial_port": "",
        "serial_baud_rate": 57600
      }
    ],
    "lighting_master": 1.0,
    "submasters": [
      {
        "group_id": "Front",
        "label": "Front",
        "level": 1.0
      }
    ],
    "blackout": false,
    "clock": {
      "bpm": 120.0,
      "beat_phase": 0.0,
      "beat_counter": 0,
      "tap_count": 0,
      "source": "Manual"
    },
    "stage_map": {
      "locked": false,
      "min_x": -6.0,
      "max_x": 6.0,
      "min_z": -4.0,
      "max_z": 6.0
    },
    "stage_map_presets": [
      {
        "label": "Mini Venue",
        "config": {
          "locked": false,
          "min_x": -6.0,
          "max_x": 6.0,
          "min_z": -4.0,
          "max_z": 6.0
        },
        "stage_objects": [
          {
            "id": 1,
            "label": "Main Deck",
            "kind": "Stage",
            "x": 0.0,
            "z": 1.0,
            "width": 10.0,
            "depth": 6.0,
            "rotation_deg": 0.0,
            "color": "#5dd64c"
          },
          {
            "id": 2,
            "label": "Front Truss",
            "kind": "Truss",
            "x": 0.0,
            "z": -2.4,
            "width": 8.5,
            "depth": 0.35,
            "rotation_deg": 0.0,
            "color": "#f2c14e"
          },
          {
            "id": 3,
            "label": "Projection Screen",
            "kind": "Screen",
            "x": 0.0,
            "z": 4.0,
            "width": 5.6,
            "depth": 0.25,
            "rotation_deg": 0.0,
            "color": "#4cb7ff"
          }
        ]
      }
    ],
    "stage_objects": [
      {
        "id": 1,
        "label": "Main Deck",
        "kind": "Stage",
        "x": 0.0,
        "z": 1.0,
        "width": 10.0,
        "depth": 6.0,
        "rotation_deg": 0.0,
        "color": "#5dd64c"
      },
      {
        "id": 2,
        "label": "Front Truss",
        "kind": "Truss",
        "x": 0.0,
        "z": -2.4,
        "width": 8.5,
        "depth": 0.35,
        "rotation_deg": 0.0,
        "color": "#f2c14e"
      },
      {
        "id": 3,
        "label": "Projection Screen",
        "kind": "Screen",
        "x": 0.0,
        "z": 4.0,
        "width": 5.6,
        "depth": 0.25,
        "rotation_deg": 0.0,
        "color": "#4cb7ff"
      }
    ],
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
      "tick_jitter_stddev_us": 0.0,
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
