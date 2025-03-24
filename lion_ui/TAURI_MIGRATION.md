# Tauri 1.x to 2.x Migration Summary

This document outlines the changes made to migrate the Lion UI project from Tauri 1.x to Tauri 2.x.

## Key Findings

After several attempts, we found that:

1. The Cargo.toml dependencies needed to be updated - Tauri 2.0 stable has different feature flags than the beta/alpha versions
2. The configuration file format has changed significantly
3. The API for system tray and window management has been updated

## Configuration Changes

### 1. `tauri.conf.json` Updates

We simplified the configuration file to a minimal working version:

```json
{
  "build": {
    "beforeBuildCommand": "",
    "frontendDist": "../frontend/dist"
  },
  "productName": "Lion UI",
  "version": "0.1.0",
  "identifier": "com.lion.ui"
}
```

The key changes from Tauri 1.x are:
- `distDir` is now `frontendDist`
- `systemTray` is now `trayIcon` (though we removed it from the minimal config)
- Some properties like `allowlist` have been removed or renamed

### 2. Cargo Dependencies

We discovered a critical issue with feature flags:
- Tauri 2.0 stable doesn't have the `"api"` or `"tray-icon"` features
- We had to remove these features from both Cargo.toml files
- Updated to use `version = "^2.0"` without specifying any features

```toml
# In src-tauri/Cargo.toml
tauri = { version = "^2.0" }

# In main Cargo.toml
[dependencies.tauri]
version = "^2.0"
optional = true
```

## Code Changes

### 1. System Tray Implementation

We updated the system tray implementation in `main.rs` with these changes:

- Changed `SystemTrayMenu` to `Menu`
- Changed `CustomMenuItem` to `MenuItem`
- Changed `SystemTrayMenuItem::Separator` to `add_separator()`
- Changed `TrayIcon::new()` to `TrayIconBuilder::new()`
- Changed `.with_menu(tray_menu)` to `.menu(tray_menu)`
- Changed `.plugin(tray_icon)` to `.tray_icon(tray_icon.build().unwrap())`

### 2. Event Emission in `bridge.rs`

Updated the event emission API:

- Changed `window.emit(event_name, payload)` to `window.emit_to(&window.label(), event_name, payload)`
  which is the recommended approach in Tauri 2.x

### 3. URI Protocol Registration

Updated custom URI protocol registration:

- Removed the plugin-based approach
- Used the direct `.register_uri_scheme_protocol()` method on the Builder
- Updated response creation to use `ResponseBuilder`

## Next Steps

1. **Test with the simplified configuration**: First, verify that the application starts with the minimal configuration.

2. **Gradually re-add functionality**: Once the basic application works, you can gradually re-add elements to the configuration like the system tray, windows, etc.

3. **Update source code**: Review any code that uses Tauri APIs and ensure it's compatible with the new API structure.

4. **Consider using Tauri CLI**: To create a template configuration:
   ```bash
   cargo tauri init
   ```

   This will generate a valid template that matches your installed Tauri version.

5. **Verify frontend event listeners**: Make sure that any frontend code that listens for Tauri events is updated to match the new event naming pattern.

6. **Update JavaScript dependencies**: If using `@tauri-apps/api` in the frontend:
   
   ```bash
   npm install @tauri-apps/api@^2.0.0-beta
   # or
   yarn add @tauri-apps/api@^2.0.0-beta
   # or 
   pnpm add @tauri-apps/api@^2.0.0-beta
   ```

## Reference Links

- [Official Tauri 2.x Migration Guide](https://tauri.app/v2/migration)
- [Tauri 2.x API Reference](https://docs.rs/tauri/2.0.0)
- [Tauri 2.x Configuration Reference](https://tauri.app/v2/api/config)
- [Tauri 2.x Features List](https://tauri.app/v2/api/features-and-attributes)