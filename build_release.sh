#!/bin/bash
set -e

# 1. Build the app (standard build)
echo "📦 Building Tauri app..."
cargo tauri build

# 2. Modify the .app bundle
APP_PATH="target/release/bundle/macos/iTunnel.app"
MACOS_DIR="$APP_PATH/Contents/MacOS"

echo "🔧 Injecting root privilege wrapper..."

# Check if already renamed (in case of re-run without clean)
if [ -f "$MACOS_DIR/iTunnel-bin" ]; then
    echo "  Binary already renamed, skipping rename."
else
    mv "$MACOS_DIR/iTunnel" "$MACOS_DIR/iTunnel-bin"
fi

# Create wrapper script
cat <<EOF > "$MACOS_DIR/iTunnel"
#!/bin/bash
DIR=\$(cd "\$(dirname "\$0")"; pwd)
/usr/bin/osascript -e "do shell script \"'\$DIR/iTunnel-bin' > /tmp/itunnel.log 2>&1 &\" with administrator privileges"
EOF

chmod +x "$MACOS_DIR/iTunnel"

# 3. Create new DMG
# 3. Create new DMG
echo "💿 Creating new DMG with drag-and-drop..."
DMG_NAME="iTunnel_Root_0.1.0_x64.dmg"
DMG_DIR="target/release/bundle/dmg"
DMG_PATH="$DMG_DIR/$DMG_NAME"
DMG_SOURCE="$DMG_DIR/source"

echo "  DMG Path: $DMG_PATH"
echo "  Source Dir: $DMG_SOURCE"

# Prepare source directory for DMG
echo "  Preparing source directory..."
rm -rf "$DMG_SOURCE"
mkdir -p "$DMG_SOURCE"
cp -R "$APP_PATH" "$DMG_SOURCE/"
ln -s /Applications "$DMG_SOURCE/Applications"

echo "  Source content:"
ls -l "$DMG_SOURCE"
echo "  MacOS Directory Content in DMG Source:"
ls -l "$DMG_SOURCE/iTunnel.app/Contents/MacOS/"

# Remove old DMG if exists
if [ -f "$DMG_PATH" ]; then
    echo "  Removing existing DMG..."
    rm -f "$DMG_PATH"
fi

# Use hdiutil to create DMG
echo "  Running hdiutil..."
hdiutil create -volname "iTunnel" -srcfolder "$DMG_SOURCE" -ov -format UDZO "$DMG_PATH"

if [ -f "$DMG_PATH" ]; then
    echo "✅ DMG created successfully at: $DMG_PATH"
    ls -lh "$DMG_PATH"
else
    echo "❌ Error: DMG file not found at expected path!"
    exit 1
fi

# Clean up
echo "  Cleaning up source directory..."
rm -rf "$DMG_SOURCE"

echo ""
echo "🎉 Build Complete!"
