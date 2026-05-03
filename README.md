## OPENCOM

This is an open source project for communicating with OpCom/VauxCom OBDII adapters for Opel/Vauxhall vehicles.

It is currently being developed for an Opel Astra G 1.4

The software can be used with the acquire sub-command to read live data from the adapter. You can save the raw timestamped responses as a JSON file for later decoding with the -a flag and/or use -p to print an NDJSON of the decoded values to stdout.

It is possible to read raw data saved from an acquire and decode them with the decode sub-command.

The decoded NDJSON object looks like the following

```json
{
  "throttle_position_percentage": 6.6666665,
  "throttle_position_voltage": 0.8775,
  "battery_voltage": 13.8,
  "air_fuel_ratio": 14.6,
  "idle_air_control_valve_percentage": 6.6666665,
  "injection_pulse_timing_milliseconds": 2.15,
  "o2_block_learn_multiplier_cell_number": 2,
  "rotations_per_minute": 3175,
  "coolant_temperature_c": 95.75
}
```

The raw archive JSON looks like the following

```json
{
  "data_type": "opencom_archive_log_file",
  "data_structure_version": "0.1.0",
  "data": [
    {
      "timestamp": 1777108549.38753000,
      "command": [ 7, 0, 1, 130, 17, 241, 33, 1, 166, 84 ],
      "response": [ 3, 0, 65, 130, 0, 198, 55, 0, 251, 178, 241, 17, 97, 1, 3, 5, 5, 224, 1, 5, 33, 84, 140, 0, 105, 115, 165, 127, 23, 9, 10, 0, 0, 0, 0, 33, 0, 0, 49, 40, 3, 81, 81, 110, 128, 18, 130, 115, 0, 146, 0, 0, 66, 76, 128, 52, 162, 0, 0, 0, 1, 38, 229, 252 ]
      ]
    }, {
      "timestamp": 1777108549.38753000,
      "command": [ 7, 0, 1, 130, 17, 241, 33, 1, 166, 84 ],
      "response": [ 3, 0, 65, 130, 0, 198, 55, 0, 251, 178, 241, 17, 97, 1, 3, 5, 5, 224, 1, 5, 33, 84, 140, 0, 105, 115, 165, 127, 23, 9, 10, 0, 0, 0, 0, 33, 0, 0, 49, 40, 3, 81, 81, 110, 128, 18, 130, 115, 0, 146, 0, 0, 66, 76, 128, 52, 162, 0, 0, 0, 1, 38, 229, 252 ]
      ]
    },

    ....

  ]
}

