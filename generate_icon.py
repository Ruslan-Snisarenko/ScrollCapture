import math
from PIL import Image, ImageDraw

def create_minimalist_icon(size=256):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(size * 0.08)
    corner_r = int(size * 0.22)
    x0, y0 = pad, pad
    x1, y1 = size - pad, size - pad

    # 1. Background squircle: Dark Slate / Obsidian (#10141E) with subtle vibrant Royal Blue (#2563EB) glow/gradient
    # We draw rounded rectangle with dark background and vibrant royal blue border
    for i in range(corner_r):
        pass # Draw rounded rect
    
    # Outer background
    draw.rounded_rectangle([x0, y0, x1, y1], radius=corner_r, fill=(16, 20, 30, 255), outline=(37, 99, 235, 255), width=int(size * 0.03))

    # Inner subtle blue glow circle / accent
    cx, cy = size // 2, size // 2

    # 2. Minimalist Scrolling Screen symbol:
    # A sleek viewport rectangle (representing screen / document)
    vw = int(size * 0.44)
    vh = int(size * 0.52)
    vx0 = cx - vw // 2
    vy0 = cy - vh // 2
    vx1 = cx + vw // 2
    vy1 = cy + vh // 2
    v_radius = int(size * 0.06)

    # Document / Screen base
    draw.rounded_rectangle([vx0, vy0, vx1, vy1], radius=v_radius, fill=(22, 28, 44, 255), outline=(56, 189, 248, 255), width=int(size * 0.025))

    # 3. Horizontal text lines on screen (minimalist lines)
    line_x0 = vx0 + int(vw * 0.20)
    line_x1 = vx1 - int(vw * 0.20)
    l_w = max(2, int(size * 0.022))

    y_line1 = vy0 + int(vh * 0.22)
    y_line2 = vy0 + int(vh * 0.38)
    
    draw.line([line_x0, y_line1, line_x1, y_line1], fill=(148, 163, 184, 200), width=l_w)
    draw.line([line_x0, y_line2, line_x1 - int(vw * 0.25), y_line2], fill=(100, 116, 139, 180), width=l_w)

    # 4. Downward Scroll Arrow in vibrant Cyan / Blue (#38BDF8 / #00E5FF)
    arrow_y0 = vy0 + int(vh * 0.52)
    arrow_y1 = vy0 + int(vh * 0.78)
    a_stroke = max(3, int(size * 0.035))

    # Arrow shaft
    draw.line([cx, arrow_y0, cx, arrow_y1], fill=(56, 189, 248, 255), width=a_stroke)
    
    # Arrow head
    head_size = int(size * 0.08)
    draw.line([cx - head_size, arrow_y1 - head_size, cx, arrow_y1], fill=(56, 189, 248, 255), width=a_stroke)
    draw.line([cx + head_size, arrow_y1 - head_size, cx, arrow_y1], fill=(56, 189, 248, 255), width=a_stroke)

    # 5. Little camera / crop dot in top right
    dot_r = int(size * 0.035)
    dot_x = vx1 - int(vw * 0.18)
    dot_y = vy0 + int(vh * 0.18)
    draw.ellipse([dot_x - dot_r, dot_y - dot_r, dot_x + dot_r, dot_y + dot_r], fill=(37, 99, 235, 255))

    return img

# Generate multi-size ICO
sizes = [(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)]
images = [create_minimalist_icon(s[0]) for s in sizes]

ico_path = r"C:\Users\rusua\Desktop\Projects\ScrollCapture_Rust\app_icon.ico"
png_path = r"C:\Users\rusua\Desktop\Projects\ScrollCapture_Rust\app_icon.png"

images[0].save(png_path)
images[0].save(ico_path, format="ICO", sizes=sizes)
print("Icon generated successfully!")
