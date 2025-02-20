# Weather Plugin

A plugin for the lion microkernel system that provides weather information for cities worldwide.

## Features

- Get current weather conditions for any city
- Get multi-day weather forecasts
- Detailed weather information including:
  - Temperature
  - Humidity
  - Wind speed and direction
  - Weather conditions (sunny, cloudy, rain, etc.)
  - Atmospheric pressure

## Functions

### current
Gets the current weather conditions for a specified city.

Input format:
```json
{
    "function": "current",
    "args": {
        "city": "San Francisco",
        "country": "US"  // Optional: helps disambiguate cities with same name
    }
}
```

Example response:
```json
{
    "location": {
        "city": "San Francisco",
        "country": "US",
        "coordinates": {
            "lat": 37.7749,
            "lon": -122.4194
        }
    },
    "current": {
        "temperature": 18.5,
        "humidity": 72,
        "conditions": "Partly cloudy",
        "wind": {
            "speed": 12.5,
            "direction": "NW"
        },
        "pressure": 1015
    },
    "units": {
        "temperature": "celsius",
        "wind_speed": "km/h",
        "pressure": "hPa"
    }
}
```

### forecast
Gets a weather forecast for the next several days.

Input format:
```json
{
    "function": "forecast",
    "args": {
        "city": "San Francisco",
        "country": "US",  // Optional
        "days": 5        // Optional: number of days (1-7, default: 5)
    }
}
```

Example response:
```json
{
    "location": {
        "city": "San Francisco",
        "country": "US"
    },
    "forecast": [
        {
            "date": "2025-02-20",
            "temperature": {
                "min": 12.5,
                "max": 19.8
            },
            "conditions": "Sunny",
            "precipitation_chance": 10
        },
        // Additional days...
    ],
    "units": {
        "temperature": "celsius",
        "precipitation": "percent"
    }
}
```

## Implementation Notes

- Uses a reliable weather data API for accurate information
- Caches responses briefly to avoid redundant API calls
- Handles city name resolution and disambiguation
- Provides consistent unit formats (metric by default)

## Error Handling

The plugin returns clear error messages for:
- City not found
- Invalid country codes
- Network connectivity issues
- API rate limiting or service issues
- Invalid number of forecast days

## Permissions

This plugin requires the "net" permission to:
- Make HTTP requests to weather API
- Resolve city names to coordinates
- Fetch weather data and forecasts
