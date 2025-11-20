#!/usr/bin/env python3
"""
Generate all icon files for AuroraHeart from logo.png
"""

import os
from PIL import Image

# Source logo
LOGO_PATH = "logo.png"
ICONS_DIR = "crates/aurora-ui/icons"

# Icon sizes needed for Tauri
ICON_SIZES = [32, 128, 256, 512]

def ensure_dir(directory):
    """Create directory if it doesn't exist"""
    os.makedirs(directory, exist_ok=True)

def generate_png_icons(logo_image):
    """Generate PNG icons in various sizes"""
    print("Generating PNG icons...")
    ensure_dir(ICONS_DIR)

    for size in ICON_SIZES:
        output_path = os.path.join(ICONS_DIR, f"{size}x{size}.png")
        resized = logo_image.resize((size, size), Image.Resampling.LANCZOS)
        resized.save(output_path, "PNG")
        print(f"  ✓ Generated {output_path}")

    # Main icon.png (1024x1024)
    main_icon = logo_image.resize((1024, 1024), Image.Resampling.LANCZOS)
    main_icon_path = os.path.join(ICONS_DIR, "icon.png")
    main_icon.save(main_icon_path, "PNG")
    print(f"  ✓ Generated {main_icon_path}")

def generate_ico(logo_image):
    """Generate Windows .ico file with multiple sizes"""
    print("Generating Windows .ico file...")

    ico_path = os.path.join(ICONS_DIR, "icon.ico")

    # Create images at multiple sizes for the ICO
    sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    images = []
    for size in sizes:
        img = logo_image.resize(size, Image.Resampling.LANCZOS)
        images.append(img)

    # Save as ICO with multiple sizes
    images[0].save(ico_path, format='ICO', sizes=sizes, append_images=images[1:])
    print(f"  ✓ Generated {ico_path}")

def generate_web_icons(logo_image):
    """Generate web icons (favicon, etc.)"""
    print("Generating web icons...")
    public_dir = "crates/aurora-ui/src-ui/public"
    ensure_dir(public_dir)

    # Favicon (32x32)
    favicon = logo_image.resize((32, 32), Image.Resampling.LANCZOS)
    favicon_path = os.path.join(public_dir, "favicon.ico")
    favicon.save(favicon_path, format='ICO', sizes=[(16, 16), (32, 32)])
    print(f"  ✓ Generated {favicon_path}")

    # Apple touch icon (180x180)
    apple_icon = logo_image.resize((180, 180), Image.Resampling.LANCZOS)
    apple_icon_path = os.path.join(public_dir, "apple-touch-icon.png")
    apple_icon.save(apple_icon_path, "PNG")
    print(f"  ✓ Generated {apple_icon_path}")

    # Web app icon (512x512)
    web_icon = logo_image.resize((512, 512), Image.Resampling.LANCZOS)
    web_icon_path = os.path.join(public_dir, "logo512.png")
    web_icon.save(web_icon_path, "PNG")
    print(f"  ✓ Generated {web_icon_path}")

def main():
    print("AuroraHeart Icon Generator")
    print("=" * 50)

    if not os.path.exists(LOGO_PATH):
        print(f"Error: {LOGO_PATH} not found!")
        return 1

    # Open logo
    print(f"Loading {LOGO_PATH}...")
    logo = Image.open(LOGO_PATH)

    # Ensure RGBA mode for transparency
    if logo.mode != 'RGBA':
        logo = logo.convert('RGBA')

    print(f"Logo size: {logo.size[0]}x{logo.size[1]}")
    print()

    # Generate all icon types
    generate_png_icons(logo)
    generate_ico(logo)
    generate_web_icons(logo)

    print()
    print("=" * 50)
    print("✓ All icons generated successfully!")
    print()
    print("Generated files:")
    print(f"  - {ICONS_DIR}/icon.ico (Windows)")
    print(f"  - {ICONS_DIR}/icon.png (Main icon)")
    print(f"  - {ICONS_DIR}/32x32.png through 512x512.png")
    print(f"  - crates/aurora-ui/src-ui/public/favicon.ico")
    print(f"  - crates/aurora-ui/src-ui/public/apple-touch-icon.png")
    print(f"  - crates/aurora-ui/src-ui/public/logo512.png")

    return 0

if __name__ == "__main__":
    exit(main())
