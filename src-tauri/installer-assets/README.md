# Installer Assets

This directory contains assets used by the various platform installers.

## Required Assets

### Windows (MSI/NSIS)
- `banner.bmp` - Installer banner image (493 x 58 pixels)
- `dialog.bmp` - Installer dialog image (493 x 312 pixels)
- `header.bmp` - NSIS header image (150 x 57 pixels)
- `sidebar.bmp` - NSIS sidebar image (164 x 314 pixels)

### macOS (DMG)
- `dmg-background.png` - DMG background image (540 x 380 pixels)

### Linux
- Desktop entry files are automatically generated
- MIME type associations are configured in the main tauri.conf.json

## Asset Guidelines

### Branding
- Use consistent branding with the TTRPG Assistant logo
- Maintain brand colors and visual identity
- Ensure high contrast for accessibility

### Technical Requirements
- **Windows BMP files**: Use 24-bit color depth, no compression
- **macOS PNG files**: Use PNG format with transparency support
- **File sizes**: Keep under 1MB per asset for faster downloads
- **Resolution**: Use appropriate DPI for target platform (96 DPI for Windows, 144 DPI for macOS)

### Image Specifications

#### banner.bmp (Windows MSI)
- Size: 493 x 58 pixels
- Format: 24-bit BMP
- Content: Application logo and name, horizontal layout

#### dialog.bmp (Windows MSI)
- Size: 493 x 312 pixels
- Format: 24-bit BMP
- Content: Full installer dialog with branding

#### header.bmp (Windows NSIS)
- Size: 150 x 57 pixels
- Format: 24-bit BMP
- Content: Small header logo

#### sidebar.bmp (Windows NSIS)
- Size: 164 x 314 pixels
- Format: 24-bit BMP
- Content: Vertical sidebar with branding

#### dmg-background.png (macOS DMG)
- Size: 540 x 380 pixels
- Format: PNG with alpha channel
- Content: Background image for DMG installer window

## Creating Assets

1. **Design Tools**: Use professional design tools (Adobe Illustrator, Figma, etc.)
2. **Export Settings**: Follow exact pixel dimensions and color depth requirements
3. **Testing**: Test installers with assets on target platforms
4. **Optimization**: Compress images while maintaining quality

## Placeholder Assets

If assets are missing, the build process will:
1. Generate placeholder assets automatically
2. Issue warnings about missing professional assets
3. Continue with the build process

To add your assets:
1. Create assets following the specifications above
2. Place them in this directory with exact filenames
3. Rebuild installers to include the new assets

## Code Signing Integration

These assets are embedded in signed installers. Ensure:
- Assets don't contain executable code
- File integrity is maintained during signing process
- Assets are included in code signing manifest