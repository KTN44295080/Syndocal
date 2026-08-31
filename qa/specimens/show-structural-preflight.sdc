{
  "version": 1,
  "app": "Syndocal",
  "dj_track_triggers": [
    {
      "id": "jinsei-over-production",
      "selector": {
        "titleContains": "人生オーバー",
        "fallbackDeck": 1
      },
      "timelineId": 1,
      "retrigger": "once_per_play_session"
    }
  ],
  "snapshot": {
    "timeline": {
      "id": 1,
      "label": "人生オーバー",
      "events": [],
      "automations": [],
      "video_automations": [],
      "loop_region": {
        "a_ms": 2000,
        "b_ms": 3400,
        "enabled": true,
        "musical_length_beats": 4
      },
      "follow": {
        "enabled": true,
        "next_timeline_id": 2,
        "duration": {
          "unit": "Bars",
          "value_milliunits": 1000
        },
        "curve": "Linear",
        "video_kind": "Crossfade",
        "lighting_policy": "hold_then_cut",
        "destination_bpm": null,
        "preroll_ms": 0,
        "trans_cadence_bars": 4,
        "destination_start_mode": "wait_for_pedal",
        "hold_first_destination_measure": false,
        "fault_policy": "hold"
      },
      "tempo_meter_map_version": 1,
      "tempo_meter_map": [
        {
          "position_sixteenth_steps": 0,
          "bpm": 170,
          "numerator": 4,
          "denominator": 4,
          "interpolation": "Step",
          "measure_number": 1
        }
      ],
      "playing": false,
      "position_ms": 0,
      "duration_ms": 10000
    },
    "timeline_bank": [
      {
        "id": 1,
        "label": "人生オーバー",
        "events": [],
        "automations": [],
        "video_automations": [],
        "loop_region": {
          "a_ms": 2000,
          "b_ms": 3400,
          "enabled": true,
          "musical_length_beats": 4
        },
        "follow": {
          "enabled": true,
          "next_timeline_id": 2,
          "duration": {
            "unit": "Bars",
            "value_milliunits": 1000
          },
          "curve": "Linear",
          "video_kind": "Crossfade",
          "lighting_policy": "hold_then_cut",
          "destination_bpm": null,
          "preroll_ms": 0,
          "trans_cadence_bars": 4,
          "destination_start_mode": "wait_for_pedal",
          "hold_first_destination_measure": false,
          "fault_policy": "hold"
        },
        "tempo_meter_map_version": 1,
        "tempo_meter_map": [
          {
            "position_sixteenth_steps": 0,
            "bpm": 170,
            "numerator": 4,
            "denominator": 4,
            "interpolation": "Step",
            "measure_number": 1
          }
        ],
        "playing": false,
        "position_ms": 0,
        "duration_ms": 10000
      },
      {
        "id": 2,
        "label": "惑う星",
        "events": [],
        "automations": [],
        "video_automations": [],
        "tempo_meter_map_version": 1,
        "tempo_meter_map": [
          {
            "position_sixteenth_steps": 0,
            "bpm": 194,
            "numerator": 5,
            "denominator": 4,
            "interpolation": "Step",
            "measure_number": 1
          }
        ],
        "playing": false,
        "position_ms": 0,
        "duration_ms": 2000
      }
    ]
  }
}
