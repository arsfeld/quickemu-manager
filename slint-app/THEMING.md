# Slint Theming Guide

## Current Implementation

The Slint frontend currently uses Slint's standard widgets with a custom color theme defined in `ui/styles/theme.slint`. This provides:

- Light/Dark mode switching
- Consistent appearance across all platforms
- Custom color palette that matches the application branding

## Platform Styles Available

Slint supports several built-in styles that can make the app look more native:

### 1. **Native Style** (default)
- Attempts to match the platform's native look
- Falls back to a generic style if platform detection fails

### 2. **Fluent Style**
- Windows 11 Fluent Design System
- Rounded corners, subtle animations, Mica-like effects
- Best for Windows 11/10

### 3. **Cupertino Style**
- macOS/iOS style following Apple's Human Interface Guidelines
- Native macOS appearance with proper spacing and controls
- Best for macOS

### 4. **Material Style**
- Google's Material Design 3
- Works well on Android and GNOME-based Linux desktops
- Modern, clean appearance

### 5. **Cosmic Style**
- System76's COSMIC desktop style
- Available on Linux when COSMIC libraries are present
- Modern Linux-native appearance

## How Styles Affect Appearance

The style affects:
- **Button appearance**: Shape, shadows, hover effects
- **Scrollbars**: Native vs custom appearance
- **Input fields**: Border styles, focus indicators
- **Colors**: May override some custom colors with system colors
- **Animations**: Platform-specific animation timing
- **Spacing**: Platform-appropriate padding and margins

## Current Configuration

The app currently detects the platform and sets an appropriate style:
- **macOS**: Cupertino style
- **Windows**: Fluent style
- **Linux GNOME**: Material style
- **Linux KDE**: Fluent style
- **Other**: Native style fallback

## Customization Options

### 1. Force a Specific Style
Set the `SLINT_STYLE` environment variable:
```bash
SLINT_STYLE=material just run-slint
SLINT_STYLE=fluent just run-slint
SLINT_STYLE=cupertino just run-slint
```

### 2. Custom Widget Styling
You can create completely custom widgets instead of using std-widgets:
```slint
component CustomButton inherits Rectangle {
    // Completely custom button implementation
}
```

### 3. Hybrid Approach
Mix standard widgets with custom styling:
- Use standard widgets for complex controls (ScrollView)
- Create custom widgets for app-specific elements (VmCard)

## Platform-Specific Considerations

### Linux
- GTK4 app uses Adwaita/libadwaita for true GNOME integration
- Slint app provides more flexibility but less native integration
- Consider user's desktop environment when choosing style

### macOS
- Cupertino style provides good native appearance
- May need adjustments for macOS-specific features (traffic lights, etc.)

### Windows
- Fluent style matches Windows 11 well
- Older Windows versions might prefer Material style

## Comparison with GTK4 Frontend

| Aspect | GTK4 Frontend | Slint Frontend |
|--------|--------------|----------------|
| Linux Native | Perfect (Adwaita) | Good (Material/Fluent) |
| macOS Native | Poor (GTK on macOS) | Good (Cupertino) |
| Windows Native | Poor (GTK on Windows) | Good (Fluent) |
| Custom Theming | Limited by GTK | Full control |
| Dark Mode | System integration | Manual toggle |
| Performance | Native toolkit | Rendered by Slint |

## Future Improvements

1. **System Theme Detection**
   - Detect system dark/light preference
   - Respond to system theme changes

2. **Custom Theme Engine**
   - Allow users to create custom themes
   - Load themes from configuration files

3. **Per-Platform Optimizations**
   - Platform-specific UI adjustments
   - Native platform features integration

4. **Accessibility**
   - High contrast themes
   - Larger text options
   - Better keyboard navigation

## Testing Different Styles

To see how the app looks with different styles:

```bash
# Material Design (Google style)
SLINT_STYLE=material cargo run

# Fluent Design (Windows 11 style)  
SLINT_STYLE=fluent cargo run

# Cupertino (macOS style)
SLINT_STYLE=cupertino cargo run

# Native (platform default)
SLINT_STYLE=native cargo run
```

Note: Some styles may require additional system libraries or fonts to render correctly.