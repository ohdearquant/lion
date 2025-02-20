# Calculator Plugin

A plugin for the lion microkernel system that provides advanced mathematical calculations and unit conversions.

## Features

- Evaluate complex mathematical expressions
- Convert between different units of measurement
- Solve equations and systems of equations
- Support for:
  - Basic arithmetic
  - Trigonometric functions
  - Logarithms and exponentials
  - Unit conversions
  - Equation solving

## Functions

### calculate
Evaluates mathematical expressions with support for variables and functions.

Input format:
```json
{
    "function": "calculate",
    "args": {
        "expression": "sin(45) * sqrt(16) + log(100)",
        "variables": {  // Optional: define variables
            "x": 5,
            "y": 10
        }
    }
}
```

Example response:
```json
{
    "result": 4.8242,
    "steps": [
        "sin(45) = 0.7071",
        "sqrt(16) = 4",
        "log(100) = 2",
        "0.7071 * 4 + 2 = 4.8242"
    ]
}
```

Supported operations:
- Basic: +, -, *, /, ^, %
- Functions: sin, cos, tan, sqrt, log, ln, exp
- Constants: pi, e

### convert
Converts values between different units of measurement.

Input format:
```json
{
    "function": "convert",
    "args": {
        "value": 100,
        "from": "km/h",
        "to": "mph"
    }
}
```

Example response:
```json
{
    "result": 62.1371,
    "from": {
        "value": 100,
        "unit": "km/h"
    },
    "to": {
        "value": 62.1371,
        "unit": "mph"
    }
}
```

Supported unit categories:
- Length (km, m, mi, ft, in, etc.)
- Mass (kg, g, lb, oz, etc.)
- Speed (km/h, mph, m/s, etc.)
- Temperature (°C, °F, K)
- Volume (L, gal, mL, etc.)
- Area (m², km², ft², etc.)

### solve
Solves mathematical equations or systems of equations.

Input format:
```json
{
    "function": "solve",
    "args": {
        "equation": "x^2 + 2x - 5 = 0",
        // Or for systems of equations:
        "equations": [
            "2x + y = 5",
            "x - y = 1"
        ]
    }
}
```

Example response:
```json
{
    "solutions": {
        "x": [-3.4142, 1.4142]  // For quadratic equation
    },
    "steps": [
        "Standard form: x^2 + 2x - 5 = 0",
        "Using quadratic formula: (-2 ± √(4 + 20)) / 2",
        "x = -3.4142 or x = 1.4142"
    ]
}
```

## Implementation Notes

- Uses high-precision floating-point arithmetic
- Provides step-by-step solution breakdowns
- Validates input expressions and equations
- Handles complex numbers when necessary
- Maintains consistent unit conversion accuracy

## Error Handling

The plugin returns clear error messages for:
- Invalid mathematical expressions
- Undefined variables
- Division by zero
- Invalid units or unit conversions
- Unsolvable equations
- Syntax errors

## Permissions

This plugin requires no special permissions as it performs purely computational tasks without external system access.

## Usage Tips

1. For complex calculations, use the step-by-step output to verify the solution path
2. When converting units, check the supported unit list to ensure compatibility
3. For equation solving, provide equations in standard mathematical notation
4. Use variables to store intermediate results in complex calculations
