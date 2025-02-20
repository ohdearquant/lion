# Time Plugin

A plugin for the lion microkernel system that handles timezone conversions and time-related operations.

## Features

- Get current time in any timezone
- Convert time between different timezones
- Supports all IANA timezone names (e.g., 'America/New_York', 'Europe/London', etc.)

## Functions

### current
Gets the current time in a specified timezone.

Input format:
```json
{
    "function": "current",
    "args": {
        "timezone": "America/New_York"
    }
}
```

Example response:
```json
{
    "time": "14:30",
    "timezone": "America/New_York",
    "offset": "-05:00"
}
```

### convert
Converts time between different timezones.

Input format:
```json
{
    "function": "convert",
    "args": {
        "time": "14:30",
        "from": "America/New_York",
        "to": "Europe/London"
    }
}
```

Example response:
```json
{
    "source": {
        "time": "14:30",
        "timezone": "America/New_York",
        "offset": "-05:00"
    },
    "converted": {
        "time": "19:30",
        "timezone": "Europe/London",
        "offset": "+00:00"
    }
}
```

## Implementation Notes

- Uses the system's timezone database for accurate conversions
- Handles daylight saving time transitions automatically
- All times are in 24-hour format for consistency
- Returns timezone offsets for clarity
- Validates timezone names against IANA database

## Error Handling

The plugin returns clear error messages for:
- Invalid timezone names
- Malformed time strings
- Missing required arguments
- System timezone database access issues

## Permissions

This plugin requires the "time" permission to:
- Access system timezone database
- Get current system time
- Perform timezone calculations
